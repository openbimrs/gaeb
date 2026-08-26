#!/usr/bin/env python3
"""Kill semantic alias/package mutations and restore source and index state."""
from pathlib import Path
import json
import shutil
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ALIAS = 'gaeb'
CANONICAL = 'openbim-gaeb'
MANIFEST = ROOT / ALIAS / "Cargo.toml"
LIB = ROOT / ALIAS / "src/lib.rs"
LOCK = ROOT / "Cargo.lock"
VERSION = tomllib.loads((ROOT / CANONICAL / "Cargo.toml").read_text())["package"]["version"]
CHECKER = ROOT / "scripts/check-alias-purity.sh"
CONTENTS = ROOT / "scripts/check-package-contents.py"
original_manifest = MANIFEST.read_bytes()
original_lib = LIB.read_bytes()
original_lock = LOCK.read_bytes()
created: list[Path] = []


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=ROOT, capture_output=True, text=True)


def alias_package(locked: bool = True) -> dict:
    command = ["cargo", "metadata"]
    if locked:
        command.append("--locked")
    command += ["--no-deps", "--format-version", "1"]
    result = run(command)
    if result.returncode != 0:
        raise AssertionError(result.stderr)
    return next(item for item in json.loads(result.stdout)["packages"] if item["name"] == ALIAS)


def dependency() -> dict:
    package = alias_package()
    return next(item for item in package["dependencies"] if item["name"] == CANONICAL)


def restore_created() -> None:
    while created:
        path = created.pop()
        result = run(["git", "reset", "-q", "--", str(path.relative_to(ROOT))])
        if result.returncode != 0:
            raise AssertionError(f"could not restore index for {path}: {result.stderr}")
        if path.is_dir():
            shutil.rmtree(path, ignore_errors=True)
        elif path.exists():
            path.unlink()


def reset() -> None:
    restore_created()
    MANIFEST.write_bytes(original_manifest)
    LIB.write_bytes(original_lib)
    LOCK.write_bytes(original_lock)


def reject(name: str, assertion=None) -> None:
    if assertion is not None:
        assertion()
    result = run([str(CHECKER)])
    if result.returncode == 0:
        raise AssertionError(f"alias mutation survived: {name}")
    print(f"{name}: killed")
    reset()


def assert_true(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def append_lib_setting(setting: str) -> None:
    text = MANIFEST.read_text()
    assert_true("[lib]" not in text, "alias unexpectedly has [lib]")
    MANIFEST.write_text(text + "\n[lib]\n" + setting + "\n")


def insert_package_field(field: str) -> None:
    text = MANIFEST.read_text()
    marker = f'version = "{VERSION}"\n'
    assert_true(text.count(marker) == 1, "package version marker is not unique")
    MANIFEST.write_text(text.replace(marker, marker + field + "\n", 1))


try:
    assert_true(run([str(CHECKER)]).returncode == 0, "baseline alias checker failed")
    content_baseline = run([sys.executable, str(CONTENTS)])
    assert_true(content_baseline.returncode == 0, content_baseline.stderr)

    LIB.write_text(LIB.read_text() + "\npub struct DivergentAliasType;\n")
    reject("independent-type")

    text = MANIFEST.read_text()
    exact = f'version = "={VERSION}"'
    loose = f'version = "{VERSION}"'
    assert_true(text.count(exact) == 1, "exact dependency marker is not unique")
    MANIFEST.write_text(text.replace(exact, loose))
    reject("loose-version", lambda: assert_true(dependency()["req"] != f"={VERSION}", "loose mutation inactive"))

    text = MANIFEST.read_text()
    assert_true(text.count("[dependencies]") == 1, "dependency table is not unique")
    MANIFEST.write_text(text.replace("[dependencies]", "[target.'cfg(unix)'.dependencies]"))
    reject("target-qualified", lambda: assert_true(dependency().get("target") is not None, "target mutation inactive"))

    text = MANIFEST.read_text()
    needle = CANONICAL + " = {"
    assert_true(text.count(needle) == 1, "canonical dependency marker is not unique")
    MANIFEST.write_text(text.replace(needle, CANONICAL + " = { default-features = false,", 1))
    reject("default-features-disabled", lambda: assert_true(not dependency()["uses_default_features"], "default-feature mutation inactive"))

    text = MANIFEST.read_text()
    needle = CANONICAL + " = {"
    MANIFEST.write_text(text.replace(needle, CANONICAL + " = { optional = true,", 1))
    reject("optional-dependency", lambda: assert_true(dependency()["optional"], "optional mutation inactive"))

    with MANIFEST.open("a") as stream:
        stream.write('serde = "1"\n')
    reject("extra-dependency", lambda: assert_true(len(alias_package()["dependencies"]) >= 2, "extra dependency mutation inactive"))

    with MANIFEST.open("a") as stream:
        stream.write('\n[features]\nprobe = []\n')
    reject("alias-feature", lambda: assert_true("probe" in alias_package()["features"], "feature mutation inactive"))

    append_lib_setting("doc = false")
    reject("library-doc-disabled", lambda: assert_true(alias_package()["targets"][0]["doc"] is False, "doc mutation inactive"))

    append_lib_setting("doctest = false")
    reject("library-doctest-disabled", lambda: assert_true(alias_package()["targets"][0]["doctest"] is False, "doctest mutation inactive"))

    append_lib_setting("test = false")
    reject("library-test-disabled", lambda: assert_true(alias_package()["targets"][0]["test"] is False, "test mutation inactive"))

    append_lib_setting('crate-type = ["rlib"]')
    reject("library-crate-type", lambda: assert_true(alias_package()["targets"][0]["crate_types"] == ["rlib"], "crate-type mutation inactive"))

    text = MANIFEST.read_text()
    assert_true(text.count('edition = "2021"') == 1, "edition marker is not unique")
    MANIFEST.write_text(text.replace('edition = "2021"', 'edition = "2018"'))
    reject("package-edition", lambda: assert_true(alias_package()["edition"] == "2018", "edition mutation inactive"))

    text = MANIFEST.read_text()
    marker = 'rust-version = "1.85"\n'
    assert_true(text.count(marker) == 1, "rust-version marker is not unique")
    MANIFEST.write_text(text.replace(marker, ""))
    reject("package-rust-version", lambda: assert_true(alias_package()["rust_version"] is None, "rust-version mutation inactive"))

    text = MANIFEST.read_text()
    marker = "publish = false\n"
    assert_true(text.count(marker) == 1, "publish marker is not unique")
    MANIFEST.write_text(text.replace(marker, ""))
    reject("package-publish-enabled", lambda: assert_true(alias_package()["publish"] is None, "publish mutation inactive"))

    text = MANIFEST.read_text()
    marker = f'version = "{VERSION}"'
    assert_true(text.count(marker) == 1, "alias version marker is not unique")
    mutated_version = f"{VERSION}-mutation"
    MANIFEST.write_text(text.replace(marker, f'version = "{mutated_version}"', 1))
    updated = alias_package(locked=False)
    assert_true(updated["version"] == mutated_version, "version mutation inactive")
    reject("package-version")

    insert_package_field('links = "alias-contract-probe"')
    build_rs = ROOT / ALIAS / "build.rs"
    build_rs.write_text("fn main() {}\n")
    created.append(build_rs)
    reject("native-links", lambda: assert_true(alias_package()["links"] == "alias-contract-probe", "links mutation inactive"))

    bin_dir = ROOT / ALIAS / "src/bin"
    bin_dir.mkdir()
    created.append(bin_dir)
    (bin_dir / "probe.rs").write_text("fn main() {}\n")
    reject("binary-target", lambda: assert_true(len(alias_package()["targets"]) >= 2, "binary mutation inactive"))

    build_rs = ROOT / ALIAS / "build.rs"
    build_rs.write_text("fn main() {}\n")
    created.append(build_rs)
    reject("build-script", lambda: assert_true(any("custom-build" in target["kind"] for target in alias_package()["targets"]), "build mutation inactive"))

    payload = ROOT / ALIAS / "src/payload.txt"
    payload.write_text("unexpected package payload\n")
    created.append(payload)
    staged = run(["git", "add", "-f", str(payload.relative_to(ROOT))])
    assert_true(staged.returncode == 0, "could not stage payload mutation")
    listed = run(["cargo", "package", "--locked", "--allow-dirty", "--list", "-p", ALIAS])
    assert_true("src/payload.txt" in listed.stdout.splitlines(), "payload mutation did not enter package inventory")
    result = run([sys.executable, str(CONTENTS)])
    assert_true(result.returncode != 0, "unexpected package payload survived")
    print("unexpected-package-payload: killed")
    reset()

    assert_true(MANIFEST.read_bytes() == original_manifest, "manifest was not restored")
    assert_true(LIB.read_bytes() == original_lib, "source was not restored")
    assert_true(LOCK.read_bytes() == original_lock, "lockfile was not restored")
    assert_true(run([str(CHECKER)]).returncode == 0, "restored alias baseline failed")
    assert_true(run([sys.executable, str(CONTENTS)]).returncode == 0, "restored package baseline failed")
finally:
    reset()

print("alias contract mutations: 19/19 killed; source, lockfile, and index restored")
