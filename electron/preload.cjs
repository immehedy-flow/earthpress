const { contextBridge, ipcRenderer, webUtils } = require('electron');

contextBridge.exposeInMainWorld('earthpress', {
  selectImages: () => ipcRenderer.invoke('select-images'),
  inspectFiles: (filePaths) => ipcRenderer.invoke('inspect-files', filePaths),
  selectOutputDir: () => ipcRenderer.invoke('select-output-dir'),
  convertImages: (files, options) => ipcRenderer.invoke('convert-images', { files, options }),
  revealPath: (filePath) => ipcRenderer.invoke('reveal-path', filePath),
  getPathForFile: (file) => webUtils.getPathForFile(file)
});
