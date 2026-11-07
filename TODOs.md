# aricma-todo todos


- [ ] (B) we need to refine the README
- [x] (A) need to fix the CI
- [x] (B) need to add windows to the CI envs
- [ ] (A) the pty screen cases are unix-only: under ConPTY the child dies at startup
      (STATUS_DLL_INIT_FAILED, 0xC0000142) or draws nothing, so the TUI on Windows is
      unproven by the screen suite and has only been seen to start
- [x] (B) add a release pipeline
- [ ] (B) the pipeline releases `tui` only; `cli` and `lib` have their own versions
      and changelogs now, but a `cli-*` or `lib-*` tag is refused until they are wired up
- [ ] (C) add release docs
- [ ] (B) an install.sh per package: `cli` and `tui` release a binary each, and a
      person still has to unpack a `.tar.gz` by hand
- [ ] (C) the `v0.1.0` tag predates per-package tags; it is the only tag that does
      not name a package, so it cannot be released by the pipeline as it stands
