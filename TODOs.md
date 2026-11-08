# aricma-todo todos


- [ ] (B) we need to refine the README
- [ ] (B) add a achore/grant pipeline so that we do not use anything that our license does not allow for, we want to stay open source and do not violate any other closed licesnes
- [ ] (C) add a gitleaks pipeline that uses a git leaks template that we can run one the last commit or oif no commit is given runs on the entire history and flags all secrets we might expose, we run the step for the current commit in ci and we prepare the pipeline as a manual trigger that can be run manually if needed
- [x] (A) need to fix the CI
- [x] (B) need to add windows to the CI envs
- [ ] (A) the pty screen cases are unix-only: under ConPTY the child dies at startup
      (STATUS_DLL_INIT_FAILED, 0xC0000142) or draws nothing, so the TUI on Windows is
      unproven by the screen suite and has only been seen to start
- [x] (B) add a release pipeline
- [ ] (B) the pipeline releases `tui` only, and the input is a bare version, so
      another package needs its own workflow: `cli` and `lib` have versions and
      changelogs already, but nothing builds or tags them
- [ ] (C) add release docs
- [ ] (B) an install.sh per package: `cli` and `tui` release a binary each, and a
      person still has to unpack a `.tar.gz` by hand
- [x] (C) the `v0.1.0` tag predates per-package tags; it is the only tag that does
      not name a package, so it cannot be released by the pipeline as it stands — gone,
      the remote carries no tags at all now
- [ ] add git-cliff to fromat --check the changelog in CI
- [ ] add a PR comment for every PR we check if changelog needs to be written during CI
- [ ] add CI step to check basic release pipeline interface like files paths references etc.
- [ ] add install.sh for tui
- [ ] release 0.1.0 for tui on cargo + binaries on github — the binaries half is done and
      shipped by hand once; cargo is blocked before any pipeline can help. A
      `cargo publish --dry-run` step was tried in the release pipeline and removed again,
      because it fails at the same place CI and a laptop both do: `lazytodo` because
      `aricma-todo-lib` is a path dependency with no `version`, and `aricma-todo-lib`
      because `ops-lib` is a git dependency with no `version`. Needs a decision on
      whether `ops-lib` is a public crate at all, then `ops-lib` on crates.io, a `version`
      beside both dependencies, publishing in order `ops-lib` → `lib` → `tui`, and a
      credential: either a `CARGO_REGISTRY_TOKEN` secret or crates.io trusted publishing
      through OIDC, which needs no secret
