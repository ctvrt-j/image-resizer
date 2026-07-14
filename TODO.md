# TODO — v0.2.0

### Resizing
- [x] Handle the "backup already exists" case gracefully — currently re-resizing a folder fails; offer skip/overwrite instead
- [ ] Continue on per-image failure instead of stopping the whole batch; report failed images at the end (x icon instead of checkmark)
- [ ] Cancel button for a running resize job

### UI
- [x] Remove individual images from the list (small ✕ per card) before resizing
- [ ] Thumbnails in the image list
- [ ] Show total size saved after a finished job (before → after)
- [ ] Empty-state hint when the list is cleared
- [ ] Better loader bar animation (slow progress instead of jumps)

### Output options
- [ ] JPEG quality / compression setting
- [ ] Optional format conversion (e.g. everything to WebP)
- [ ] Configurable output: in-place with `original/` backup (current) vs. separate output folder

### Housekeeping
- [ ] Single source of truth for the version (script to sync `tauri.conf.json`, `package.json`, `Cargo.toml` before tagging)
- [ ] Code cleanup