# Image Resizer

> ⚠️ **Disclaimer:** In its current state this is a vibecoded app. It works, but expect rough edges — more improvements are coming in the future.

A small desktop app for batch-resizing images, built with Tauri, React and Rust.

## What it does

- Load images by dropping files or a whole folder onto the window, or via the Open Image / Open Folder buttons
- Set a target width and height (values are remembered between sessions)
- Resize all loaded images in one go — plain pixel scaling, nothing fancy
- Before anything is overwritten, the originals are moved into an `original/` subfolder next to the images, so nothing is ever lost
- A progress bar and per-image checkmarks show the resize progress in real time, with updated dimensions and file sizes as each image finishes

Supports the usual formats (JPEG, PNG, GIF, BMP, TIFF, WebP, ICO and more).

## Development

```bash
npm install
npm run tauri dev
```
