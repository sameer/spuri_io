# spuri_io  [![Build Status](https://travis-ci.org/sameer/spuri_io.svg?branch=master)](https://travis-ci.org/sameer/spuri_io)
Personal website translated from Go to Rust

## TODO
- [x] Translate templates from html/template to Askama
- [x] Implement Index
- [x] Implement About
- [x] Implement NavItems
- [x] Implement CSS File Integrity hash
- [x] Implement BlogIndex
- [ ] Implement BlogPage (next task)
  - [ ] Find way to maintain metadata for the markdown file
  - [ ] Implement author field
  - [ ] Handle timestamps better so the ctime field exists
- [x] Implement CodeArtGallery
- [x] CodeArt image resizing
- [x] State handling
- [x] Request logging
- [ ] Consult Mozilla Developer Network documentation (MDN) on best practices for site accessibility
- [ ] Consult MDN docmentation on modern HTML features like [picture](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
- [ ] Brush up on CSS and add more artistic effects to the site
- [ ] Look into cache-control for static files -- right now the server sends none
- [ ] Send Travis release-built binaries to server and automatically restart
- [ ] Start adding tests and checking code coverage
- [ ] And more...
