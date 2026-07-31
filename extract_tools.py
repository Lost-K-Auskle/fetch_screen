"""Extract ALL needed mingw tools from MSYS2 binutils package"""
import zstandard as zstd
import tarfile
import os
import shutil

NEEDED_TOOLS = ["as.exe", "windres.exe", "ar.exe", "ranlib.exe", "strip.exe", "dlltool.exe"]
zst_path = os.path.expanduser("~/Downloads/mingw-binutils.tar.zst")
extract_dir = "/tmp/mingw-tools"
os.makedirs(extract_dir, exist_ok=True)

dest_dir = r"C:\Users\lost\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"

with open(zst_path, "rb") as f:
    dctx = zstd.ZstdDecompressor()
    with dctx.stream_reader(f) as reader:
        with tarfile.open(fileobj=reader, mode="r|") as tar:
            for member in tar:
                name = member.name.lower()
                for tool in NEEDED_TOOLS:
                    if name.endswith("/" + tool):
                        print(f"Found: {member.name}")
                        tar.extract(member, extract_dir)

# Copy all found tools
for root, dirs, files in os.walk(extract_dir):
    for file in files:
        if file.lower() in NEEDED_TOOLS:
            src = os.path.join(root, file)
            dest = os.path.join(dest_dir, file)
            shutil.copy2(src, dest)
            print(f"Copied {file} -> {dest} ({os.path.getsize(dest)} bytes)")

print("\nSelf-contained directory now contains:")
for f in sorted(os.listdir(dest_dir)):
    size = os.path.getsize(os.path.join(dest_dir, f))
    print(f"  {f:40s} {size:>10,} bytes")

# Verify as.exe works
as_path = os.path.join(dest_dir, "as.exe")
if os.path.exists(as_path):
    import subprocess
    result = subprocess.run([as_path, "--version"], capture_output=True, text=True)
    print(f"\nas.exe version check: {result.stdout.split(chr(10))[0]}")
