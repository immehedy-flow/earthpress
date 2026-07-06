const fs = require('fs/promises');
const path = require('path');
const sharp = require('sharp');
const pngToIcoPackage = require('png-to-ico');
const pngToIco = pngToIcoPackage.default || pngToIcoPackage;

const buildDir = path.join(__dirname, '..', 'build');

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <rect width="512" height="512" rx="96" fill="#6f7d4e"/>
  <path d="M142 341c47-128 137-183 247-190-2 89-40 195-158 229-34 10-66 4-89-39Z" fill="#fffaf2"/>
  <path d="M173 337c50-68 109-111 184-136" fill="none" stroke="#b66a44" stroke-width="26" stroke-linecap="round"/>
  <path d="M130 356c42 31 99 38 151 15" fill="none" stroke="#2e2923" stroke-opacity=".24" stroke-width="18" stroke-linecap="round"/>
</svg>`;

async function main() {
  await fs.mkdir(buildDir, { recursive: true });
  const sizes = [16, 24, 32, 48, 64, 128, 256];
  const pngPaths = [];

  for (const size of sizes) {
    const outputPath = path.join(buildDir, `icon-${size}.png`);
    await sharp(Buffer.from(svg)).resize(size, size).png().toFile(outputPath);
    pngPaths.push(outputPath);
  }

  const ico = await pngToIco(pngPaths);
  await fs.writeFile(path.join(buildDir, 'icon.ico'), ico);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
