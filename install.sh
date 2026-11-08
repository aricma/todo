#!/bin/sh
# Installs one of the aricma tools from its GitHub release.
#
#   curl -fsSL https://aricma.app/install.sh | sh -s -- --bin lazytodo
#
# Which repository a tool comes from, and what its binary is called, is not
# decided here: aricma.app/registry.json says so, so adding a tool changes that
# file and nothing in this script. Each release must then contain:
#
#   tag        <binary>-v<version>
#   archive    <binary>-v<version>-<target>.tar.gz
#   checksums  <binary>_<version>_checksums.txt, one sha256 per archive
set -eu

registry_src="https://aricma.app/registry.json"
bin=""
version="latest"
install_dir=""
dry_run=0

fail() {
  echo "install: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
usage: install.sh --bin <name> [options]

  --bin <name>         the tool to install, for example lazytodo (required)
  --version <version>  the version to install, default: the latest release
  --install-dir <dir>  where the binary goes, default: the registry's
                       default_install_dir, or ~/.local/bin
  --registry <path|url>  where to read the tool registry, default:
                       https://aricma.app/registry.json
  --dry-run            print what would be downloaded and where it would go
  -h, --help           this text

The tool is looked up in the registry, so there is nothing to say about where
it is published.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --bin)
      [ $# -ge 2 ] || fail "--bin needs a value"
      bin=$2
      shift 2
      ;;
    --version)
      [ $# -ge 2 ] || fail "--version needs a value"
      version=$2
      shift 2
      ;;
    --install-dir)
      [ $# -ge 2 ] || fail "--install-dir needs a value"
      install_dir=$2
      shift 2
      ;;
    --registry)
      [ $# -ge 2 ] || fail "--registry needs a value"
      registry_src=$2
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      fail "unknown argument: $1"
      ;;
  esac
done

[ -n "$bin" ] || fail "--bin is required, for example --bin lazytodo"

# Every accepted value ends up in a URL or a path, so nothing but its own alphabet.
case "$bin" in
  "" | *[!a-z0-9-]*) fail "not a program name: $bin" ;;
esac
case "$version" in
  latest) ;;
  "" | *[!0-9A-Za-z.-]*) fail "not a version: $version" ;;
esac

case "$registry_src" in
  *://*) registry=$(curl -fsSL "$registry_src") || fail "cannot read $registry_src" ;;
  *)
    [ -f "$registry_src" ] || fail "no such file: $registry_src"
    registry=$(cat "$registry_src")
    ;;
esac

# One line per entry under "tools". A plain grep for brace pairs would split an
# entry whose description contains a brace, so braces inside strings do not count.
entry=$(printf '%s\n' "$registry" | awk '
  {
    n = length($0)
    for (i = 1; i <= n; i++) {
      c = substr($0, i, 1)
      if (instr) {
        if (esc) esc = 0
        else if (c == "\\") esc = 1
        else if (c == "\"") instr = 0
        if (collect) buf = buf c
        continue
      }
      if (c == "\"") { instr = 1; if (collect) buf = buf c; continue }
      if (c == "{") {
        depth++
        if (depth == 2) { collect = 1; buf = "{" }
        else if (collect) buf = buf c
        continue
      }
      if (c == "}") {
        if (collect && depth == 2) { print buf "}"; collect = 0; buf = "" }
        else if (collect) buf = buf c
        depth--
        continue
      }
      if (collect) buf = buf c
    }
    if (collect) buf = buf " "
  }' | grep "\"name\": *\"$bin\"" | head -1)
[ -n "$entry" ] || fail "$bin is not in $registry_src"

field() {
  printf '%s\n' "$entry" | sed -n "s/.*\"$1\": *\"\([^\"]*\)\".*/\1/p" | head -1
}

repo=$(field repo)
binary=$(field binary_name)
[ -n "$repo" ] || fail "$bin has no repo in $registry_src"
[ -n "$binary" ] || fail "$bin has no binary_name in $registry_src"
case "$repo" in
  *[!A-Za-z0-9._/-]*) fail "not a repository, in $registry_src: $repo" ;;
esac
case "$binary" in
  "" | *[!A-Za-z0-9._-]*) fail "not a binary name, in $registry_src: $binary" ;;
esac

if [ -z "$install_dir" ]; then
  install_dir=$(printf '%s\n' "$registry" |
    sed -n 's/.*"default_install_dir": *"\([^\"]*\)".*/\1/p' | head -1)
  # The registry writes the literal $HOME, which is expanded just below.
  # shellcheck disable=SC2016
  [ -n "$install_dir" ] || install_dir='$HOME/.local/bin'
fi
# shellcheck disable=SC2016
case "$install_dir" in
  '$HOME'/*) install_dir="${HOME}/${install_dir#\$HOME/}" ;;
  '~'/*) install_dir="${HOME}/${install_dir#\~/}" ;;
esac
# It comes from a remote file, so it may name a directory but not somewhere odd.
case "$install_dir" in
  /?*) ;;
  *) fail "install dir is not an absolute path: $install_dir" ;;
esac

os=$(uname -s)
arch=$(uname -m)

case "$os" in
  Linux) os_part=unknown-linux-musl ;;
  Darwin) os_part=apple-darwin ;;
  MINGW* | MSYS* | CYGWIN*)
    fail "Windows: download $binary-v<version>-x86_64-pc-windows-msvc.tar.gz from the release yourself"
    ;;
  *) fail "no prebuilt binary for $os" ;;
esac

case "$arch" in
  x86_64 | amd64) arch_part=x86_64 ;;
  arm64 | aarch64) arch_part=aarch64 ;;
  *) fail "no prebuilt binary for $arch" ;;
esac

target="${arch_part}-${os_part}"

if [ "$version" = "latest" ]; then
  latest=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest") ||
    fail "cannot ask $repo what its latest release is"
  tag=$(printf '%s\n' "$latest" |
    sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)
  [ -n "$tag" ] || fail "no published release at $repo"
  case "$tag" in
    "$binary"-v*) version=${tag#"$binary-v"} ;;
    *) fail "the latest release of $repo is $tag, which is not a $binary release" ;;
  esac
else
  tag="${binary}-v${version}"
fi

archive="${binary}-v${version}-${target}.tar.gz"
checksums="${binary}_${version}_checksums.txt"
base="https://github.com/$repo/releases/download/$tag"

if [ "$dry_run" = 1 ]; then
  echo "$bin is $binary from $repo, $version for $target"
  echo "would download $base/$archive"
  echo "would verify   $base/$checksums"
  echo "would install  $install_dir/$binary"
  exit 0
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "downloading $archive from $repo"
curl -fsSL -o "$tmp/$archive" "$base/$archive" ||
  fail "cannot download $base/$archive"
curl -fsSL -o "$tmp/$checksums" "$base/$checksums" ||
  fail "cannot download $base/$checksums"

# Only the line for this archive: the file lists every target, and this install
# has one of them.
awk -v want="$archive" '$2 == want' "$tmp/$checksums" >"$tmp/expected.sha256"
[ -s "$tmp/expected.sha256" ] || fail "$archive is not listed in $checksums"

echo "verifying the checksum"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmp" && sha256sum -c expected.sha256)
else
  (cd "$tmp" && shasum -a 256 -c expected.sha256)
fi

tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$install_dir"
cp "$tmp/${binary}-v${version}-${target}/$binary" "$install_dir/$binary"
chmod 755 "$install_dir/$binary"

# A downloaded program runs only after this on macOS, and the refusal reads as a
# broken binary rather than as a security setting.
if [ "$os" = "Darwin" ] && command -v xattr >/dev/null 2>&1; then
  xattr -d com.apple.quarantine "$install_dir/$binary" 2>/dev/null || true
fi

echo "installed $install_dir/$binary, $version for $target"

case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *)
    echo "note: $install_dir is not on your PATH. Add it, for example:"
    echo "  export PATH=\"$install_dir:\$PATH\""
    ;;
esac
