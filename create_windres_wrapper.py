"""Create a windres wrapper that adds --no-preprocess to skip cc1 dependency"""
import shutil, os

self_dir = r"C:\Users\lost\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"

# Write a Python wrapper script
wrapper = os.path.join(self_dir, "x86_64-w64-mingw32-windres_wrapper.py")
with open(wrapper, "w") as f:
    f.write(r'''
import sys, subprocess, os
real = os.path.join(os.path.dirname(__file__), "windres_real.exe")
args = [real]
i = 1
while i < len(sys.argv):
    a = sys.argv[i]
    if a in ("--input", "-i"):
        args.append(a)
        args.append(sys.argv[i+1])
        # Add --no-preprocess before input
        args.append("--no-preprocess")
        i += 2
    else:
        args.append(a)
        i += 1
sys.exit(subprocess.call(args))
''')

# Rename real windres
real_windres = os.path.join(self_dir, "x86_64-w64-mingw32-windres.exe")
real_backup = os.path.join(self_dir, "windres_real.exe")
if os.path.exists(real_windres) and not os.path.exists(real_backup):
    shutil.move(real_windres, real_backup)
    print(f"Backed up windres to windres_real.exe")

# Build wrapper to exe
import subprocess
subprocess.check_call([
    "pyinstaller", "--onefile", "--name", "x86_64-w64-mingw32-windres",
    "--noconsole", "--distpath", self_dir,
    wrapper
])
print("Windres wrapper created!")
