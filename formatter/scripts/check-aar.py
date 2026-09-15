#!/usr/bin/env python3
"""Check the actual packaged ELF against Cargo's unstripped Android output.

Usage: python3 scripts/check-aar.py <NDK llvm bin directory>
Run after scripts/build-aar.sh. Uses only Python's standard library and NDK tools.
"""
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import zipfile

tools = Path(sys.argv[1])
root = Path(__file__).resolve().parents[2]
raw = root / "target/aarch64-linux-android/formatter-release/libdprint_markdown_ja_formatter.so"
aar = root / "target/dprint-markdown-ja-formatter.aar"


def readelf(path, *args):
    return subprocess.check_output([str(tools / "llvm-readelf"), *args, str(path)], text=True)


def allocated_sections(path):
    data = path.read_bytes()
    sections = {}
    for match in re.finditer(
        r"\[\s*\d+\]\s+(\S+)\s+(\S+)\s+([0-9a-f]+)\s+([0-9a-f]+)\s+([0-9a-f]+)\s+\S+\s+(\S+)",
        readelf(path, "-SW"),
    ):
        name, kind, address, offset, size, flags = match.groups()
        if "A" in flags:
            start, length = int(offset, 16), int(size, 16)
            contents = b"" if kind == "NOBITS" else data[start:start + length]
            sections[name] = (kind, address, length, flags, contents)
    return sections


with tempfile.TemporaryDirectory() as temporary:
    packaged = Path(temporary) / "packaged.so"
    with zipfile.ZipFile(aar) as archive:
        packaged.write_bytes(archive.read("jni/arm64-v8a/libdprint_markdown_ja_formatter.so"))
    assert packaged.stat().st_size < raw.stat().st_size, "packaged library was not stripped"
    assert "AArch64" in readelf(packaged, "-h")
    exports = subprocess.check_output(
        [str(tools / "llvm-nm"), "--dynamic", "--defined-only", str(packaged)], text=True
    )
    assert [line.split()[-1] for line in exports.splitlines()] == ["JNI_OnLoad"], exports
    headers = readelf(packaged, "-lW")
    loads = [line.split() for line in headers.splitlines() if line.strip().startswith("LOAD ")]
    assert len(loads) == 4 and all(int(line[-1], 16) == 16384 for line in loads), headers
    assert all(int(line[1], 16) % 16384 == int(line[2], 16) % 16384 for line in loads)
    dynamic = readelf(packaged, "-dW")
    needed = re.findall(r"\(NEEDED\).*?\[(.*?)\]", dynamic)
    assert sorted(needed) == ["libc.so", "libdl.so"], dynamic
    # API 21's loader cannot consume Android packed relocations or DT_RELR.
    assert not re.search(r"\((?:ANDROID_|RELR|RELRSZ|RELRENT)", dynamic), dynamic
    before, after = allocated_sections(raw), allocated_sections(packaged)
    assert {".text", ".dynsym", ".eh_frame", ".gcc_except_table", ".note.android.ident"} <= after.keys()
    assert before == after, "strip changed a runtime section (including unwind/relocations)"
    # The Android ident note's first descriptor word is the minimum API level.
    note = after[".note.android.ident"][-1]
    assert note[12:20] == b"Android\0" and int.from_bytes(note[20:24], "little") == 21
    assert ".symtab" not in readelf(packaged, "-SW"), "static symbol table remains"
    print(f"AAR ELF checks passed: {packaged.stat().st_size} bytes; JNI_OnLoad only; "
          f"4 x 16 KiB LOAD; API 21; {needed}; {len(after)} allocated sections unchanged, including unwind")
