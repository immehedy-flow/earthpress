const { app, BrowserWindow, dialog, ipcMain, shell } = require('electron');
const path = require('path');
const fs = require('fs/promises');
const sharp = require('sharp');

const supportedExtensions = new Set(['.jpg', '.jpeg', '.png', '.webp', '.avif', '.tif', '.tiff', '.bmp']);

function createWindow() {
  const win = new BrowserWindow({
    width: 1220,
    height: 800,
    minWidth: 980,
    minHeight: 680,
    backgroundColor: '#f4efe7',
    icon: path.join(__dirname, '..', 'build', 'icon.ico'),
    title: 'EarthPress',
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    }
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    win.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    win.loadFile(path.join(__dirname, '..', 'dist', 'index.html'));
  }
}

app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

function formatFromExtension(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (ext === '.jpeg') return 'jpg';
  return ext.replace('.', '');
}

async function inspectImage(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (!supportedExtensions.has(ext)) return null;

  const [stat, metadata, previewBuffer] = await Promise.all([
    fs.stat(filePath),
    sharp(filePath).metadata(),
    sharp(filePath)
      .rotate()
      .resize({ width: 220, height: 160, fit: 'inside', withoutEnlargement: true })
      .webp({ quality: 72 })
      .toBuffer()
  ]);

  return {
    id: `${filePath}-${stat.mtimeMs}`,
    path: filePath,
    name: path.basename(filePath),
    directory: path.dirname(filePath),
    format: formatFromExtension(filePath),
    width: metadata.width || 0,
    height: metadata.height || 0,
    size: stat.size,
    preview: `data:image/webp;base64,${previewBuffer.toString('base64')}`
  };
}

async function inspectFiles(filePaths) {
  const uniquePaths = [...new Set(filePaths)];
  const inspected = await Promise.all(
    uniquePaths.map(async (filePath) => {
      try {
        return await inspectImage(filePath);
      } catch (error) {
        return {
          path: filePath,
          name: path.basename(filePath),
          error: error instanceof Error ? error.message : 'Could not read this image.'
        };
      }
    })
  );

  return inspected.filter(Boolean);
}

ipcMain.handle('select-images', async () => {
  const result = await dialog.showOpenDialog({
    title: 'Choose images',
    properties: ['openFile', 'multiSelections'],
    filters: [
      { name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp', 'avif', 'tif', 'tiff', 'bmp'] }
    ]
  });

  if (result.canceled) return [];
  return inspectFiles(result.filePaths);
});

ipcMain.handle('inspect-files', async (_event, filePaths) => {
  return inspectFiles(Array.isArray(filePaths) ? filePaths : []);
});

ipcMain.handle('select-output-dir', async () => {
  const result = await dialog.showOpenDialog({
    title: 'Choose output folder',
    properties: ['openDirectory', 'createDirectory']
  });

  if (result.canceled) return null;
  return result.filePaths[0];
});

function outputExtension(format) {
  return format === 'jpeg' ? 'jpg' : format;
}

function parseAspectRatio(preset) {
  const [width, height] = String(preset || '1:1')
    .split(':')
    .map((part) => Number(part));
  if (!width || !height) return 1;
  return width / height;
}

function aspectTarget(metadata, preset) {
  const originalWidth = metadata.width || 1;
  const originalHeight = metadata.height || 1;
  const targetRatio = parseAspectRatio(preset);
  const originalRatio = originalWidth / originalHeight;

  if (originalRatio > targetRatio) {
    return {
      width: Math.max(1, Math.round(originalHeight * targetRatio)),
      height: originalHeight
    };
  }

  return {
    width: originalWidth,
    height: Math.max(1, Math.round(originalWidth / targetRatio))
  };
}

function sharpPosition(position) {
  const allowed = new Set(['center', 'top', 'right', 'bottom', 'left', 'attention', 'entropy']);
  return allowed.has(position) ? position : 'center';
}

function fitBackground(value) {
  return /^#[0-9a-f]{6}$/i.test(value || '') ? value : '#fffaf2';
}

function uniqueOutputPath(basePath) {
  return fs
    .access(basePath)
    .then(async () => {
      const parsed = path.parse(basePath);
      for (let index = 1; index < 1000; index += 1) {
        const candidate = path.join(parsed.dir, `${parsed.name}-${index}${parsed.ext}`);
        try {
          await fs.access(candidate);
        } catch {
          return candidate;
        }
      }
      return path.join(parsed.dir, `${parsed.name}-${Date.now()}${parsed.ext}`);
    })
    .catch(() => basePath);
}

async function convertOne(file, options) {
  const targetFormat = options.format;
  const ext = outputExtension(targetFormat);
  const parsed = path.parse(file.path);
  const suffix = options.filenameSuffix || '';
  const outputDir = options.outputDir || file.directory || parsed.dir;
  const desiredPath = path.join(outputDir, `${parsed.name}${suffix}.${ext}`);
  const outputPath = options.overwrite ? desiredPath : await uniqueOutputPath(desiredPath);

  let pipeline = sharp(file.path, { animated: true }).rotate();
  const metadata = await sharp(file.path).metadata();

  const sizeMode =
    options.sizeMode || (options.cropMode === 'aspect' || options.cropMode === 'custom' ? options.cropMode : options.resizeMode !== 'original' ? 'resize' : 'original');

  if (sizeMode === 'aspect' || sizeMode === 'custom') {
    const target =
      sizeMode === 'custom'
        ? {
            width: Math.max(1, Math.round(Number(options.customWidth) || metadata.width || 1)),
            height: Math.max(1, Math.round(Number(options.customHeight) || metadata.height || 1))
          }
        : aspectTarget(metadata, options.aspectPreset);

    pipeline = pipeline.resize({
      ...target,
      fit: options.fitMode || 'cover',
      position: sharpPosition(options.cropPosition),
      background: fitBackground(options.background),
      withoutEnlargement: !options.allowUpscale
    });
  } else if (sizeMode === 'resize') {
    const resizeKind = options.resizeKind || options.resizeMode || 'max';
    const resize = {
      withoutEnlargement: !options.allowUpscale
    };
    if (resizeKind === 'width') resize.width = Number(options.resizeValue);
    if (resizeKind === 'height') resize.height = Number(options.resizeValue);
    if (resizeKind === 'max') {
      resize.width = Number(options.resizeValue);
      resize.height = Number(options.resizeValue);
      resize.fit = 'inside';
    }
    if (resizeKind === 'percent') {
      const scale = Number(options.resizeValue) / 100;
      resize.width = Math.max(1, Math.round((metadata.width || 1) * scale));
      resize.height = Math.max(1, Math.round((metadata.height || 1) * scale));
      resize.fit = 'inside';
    }
    pipeline = pipeline.resize(resize);
  }

  const quality = Math.max(1, Math.min(100, Number(options.quality) || 82));
  if (targetFormat === 'jpg') {
    pipeline = pipeline.jpeg({ quality, mozjpeg: true, progressive: true });
  } else if (targetFormat === 'png') {
    pipeline = pipeline.png({ compressionLevel: Number(options.pngCompression) || 8, palette: Boolean(options.pngPalette) });
  } else if (targetFormat === 'webp') {
    pipeline = pipeline.webp({ quality, lossless: Boolean(options.lossless) });
  } else if (targetFormat === 'avif') {
    pipeline = pipeline.avif({ quality, lossless: Boolean(options.lossless) });
  }

  if (options.preserveMetadata) {
    pipeline = pipeline.keepMetadata();
  }

  await pipeline.toFile(outputPath);
  const outputStat = await fs.stat(outputPath);

  return {
    id: file.id,
    name: file.name,
    outputPath,
    outputSize: outputStat.size,
    savedBytes: Math.max(0, file.size - outputStat.size),
    ok: true
  };
}

ipcMain.handle('convert-images', async (_event, payload) => {
  const files = Array.isArray(payload?.files) ? payload.files : [];
  const options = payload?.options || {};
  await fs.mkdir(options.outputDir, { recursive: true });

  const results = [];
  for (const file of files) {
    try {
      results.push(await convertOne(file, options));
    } catch (error) {
      results.push({
        id: file.id,
        name: file.name,
        ok: false,
        error: error instanceof Error ? error.message : 'Conversion failed.'
      });
    }
  }

  return results;
});

ipcMain.handle('reveal-path', async (_event, filePath) => {
  if (!filePath) return false;
  shell.showItemInFolder(filePath);
  return true;
});
