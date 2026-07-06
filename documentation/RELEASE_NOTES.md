# Release Notes

All notable changes to EarthPress are documented here. Versioning follows [Semantic Versioning](https://semver.org/) (MAJOR.MINOR.PATCH).

---

## v1.2.0

**Focus: cleaner, more compact UI**

### Changed
- Combined Resize and Crop into a single, unified **Size** section
- Object Fit and Focus options are now hidden unless Ratio or Custom size mode is active, reducing visual clutter
- Rarely used options moved into a collapsed **Advanced** panel
- Output Folder and Convert button are now sticky at the bottom of the right panel, always visible while scrolling
- Reduced font size, spacing, row height, and drop-zone height for a more compact layout

### Fixed
- Fixed low-contrast Add Images icon (now white on the green button, as intended)

### Added
- Per-image **Show in folder** button next to each row's delete button, available after conversion

---

## v1.1.0

**Focus: crop and aspect ratio controls**

### Added
- New Crop section with three modes: Off, Ratio, and Custom
- Aspect ratio presets with visual preview icons: 1:1, 4:5, 5:4, 4:3, 3:2, 16:9, 9:16
- Custom crop width and height inputs
- Object fit modes: Cover, Contain, Fill, Inside, Outside
- Crop focus options: Center, smart attention, smart detail, and top/right/bottom/left edges
- Background color option for Contain-style fitting

### Changed
- Rounder UI throughout — large panels now use ~28px corner radius, smaller controls ~14–24px
- Build output now goes to a separate `release-crop` folder

---

## v1.0.0

**Initial release**

### Added
- Local desktop app for Windows (EarthPress)
- Output support for PNG, JPG, WebP, and AVIF
- Batch image adding via button or drag-and-drop
- Quality slider, lossless option, resize controls, and metadata toggle
- Output folder picker, filename suffix, and overwrite protection
- Earthy/humanist UI palette with a custom app icon
- Windows NSIS installer (`.exe`)
