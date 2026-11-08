#!/bin/sh
# Installs one of the aricma tools from its GitHub release.
#
# Meant to be served from aricma.app/install.sh and shared by every tool:
#
#   curl -fsSL https://aricma.app/install.sh | sh -s -- --bin lazytodo --repo aricma/todo
#
# Nothing here is specific to one program. It expects the release to follow the
# conventions this repository's pipeline produces:
#
#   tag        <bin>-v<version>
#   archive    <bin>-v<version>-<target>.tar.gz
#   checksums  <bin>_<version>_checksums.txt, one sha256 per archive
set -eu

repo=""
bin=""
version="latest"
install_dir="${HOME}/.local/bin"
dry_run=0

die() {
  echo "install: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
usage: install.sh --bin <name> [options]

  --bin <name>         the program to install, for example lazytodo (required)
  --repo <owner/name>  the GitHub repository that releases it (required)
  --version <version>  the version to install, default: the latest release
  --install-dir <dir>  where the program goes, default: ~/.local/bin
  --dry-run            print what would be downloaded and where it would go
  -h, --help           this text
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --bin)
      [ $# -ge 2 ] || die "--bin needs a value"
      bin=$2
      shift 2
      ;;
    --repo)
      [ $# -ge 2 ] || die "--repo needs a value"
      repo=$2
      shift 2
      ;;
    --version)
      [ $# -ge 2 ] || die "--version needs a value"
      version=$2
      shift 2
      ;;
    --install-dir)
      [ $# -ge 2 ] || die "--install-dir needs a value"
      install_dir=$2
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
      die "unknown argument: $1"
      ;;
  esac
done

[ -n "$bin" ] || die "--bin is required, for example --bin lazytodo"
[ -n "$repo" ] || die "--repo is required: a release comes from a repository, for example aricma/todo"

# These three build a URL and a file path, so nothing but their own alphabet.
case "$bin" in
  "" | *[!a-z0-9-]*) die "not a program name: $bin" ;;
esac
case "$repo" in
  *[!A-Za-z0-9._/-]*) die "not a repository: $repo" ;;
esac
case "$version" in
  latest) ;;
  "" | *[!0-9A-Za-z.-]*) die "not a version: $version" ;;
esac

os=$(uname -s)
arch=$(uname -m)

case "$os" in
  Linux) os_part=unknown-linux-musl ;;
  Darwin) os_part=apple-darwin ;;
  MINGW* | MSYS* | CYGWIN*)
    die "Windows: download $bin-v<version>-x86_64-pc-windows-msvc.tar.gz from the release yourself"
    ;;
  *) die "no prebuilt binary for $os" ;;
esac

case "$arch" in
  x86_64 | amd64) arch_part=x86_64 ;;
  arm64 | aarch64) arch_part=aarch64 ;;
  *) die "no prebuilt binary for $arch" ;;
esac

target="${arch_part}-${os_part}"

if [ "$version" = "latest" ]; then
  tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" |
    sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)
  [ -n "$tag" ] || die "no published release at $repo"
  case "$tag" in
    "$bin"-v*) version=${tag#"$bin-v"} ;;
    *) die "the latest release of $repo is $tag, which is not a $bin release" ;;
  esac
else
  tag="${bin}-v${version}"
fi

archive="${bin}-v${version}-${target}.tar.gz"
checksums="${bin}_${version}_checksums.txt"
base="https://github.com/$repo/releases/download/$tag"

if [ "$dry_run" = 1 ]; then
  echo "would download $base/$archive"
  echo "would verify   $base/$checksums"
  echo "would install  $bin $version for $target into $install_dir"
  exit 0
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "downloading $archive"
curl -fsSL -o "$tmp/$archive" "$base/$archive"
curl -fsSL -o "$tmp/$checksums" "$base/$checksums"

# Only the line for this archive: the file lists every target, and this install
# has one of them.
awk -v want="$archive" '$2 == want' "$tmp/$checksums" >"$tmp/expected.sha256"
[ -s "$tmp/expected.sha256" ] || die "$archive is not listed in $checksums"

echo "verifying the checksum"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmp" && sha256sum -c expected.sha256)
else
  (cd "$tmp" && shasum -a 256 -c expected.sha256)
fi

tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$install_dir"
cp "$tmp/${bin}-v${version}-${target}/$bin" "$install_dir/$bin"
chmod 755 "$install_dir/$bin"

# A downloaded program runs only after this on macOS, and the refusal reads as a
# broken binary rather than as a security setting.
if [ "$os" = "Darwin" ] && command -v xattr >/dev/null 2>&1; then
  xattr -d com.apple.quarantine "$install_dir/$bin" 2>/dev/null || true
fi

echo "installed $install_dir/$bin, $bin $version for $target"

case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *)
    echo "note: $install_dir is not on your PATH. Add it, for example:"
    echo "  export PATH=\"$install_dir:\$PATH\""
    ;;
esac
