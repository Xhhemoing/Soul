#!/usr/bin/env python3
"""Read-only baseline probes for Soul plan polish."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterable


REFS = {
    "current": "HEAD",
    "product-lock": "origin/cursor/soul-product-lock-7b1c",
    "goal1": "origin/cursor/soul-goal1-7b1c",
    "main": "origin/main",
}

NINE_SCHEMAS = [
    "event.schema.json",
    "evidence.schema.json",
    "inference.schema.json",
    "profile.schema.json",
    "memory.schema.json",
    "contact.schema.json",
    "relationship.schema.json",
    "audit.schema.json",
    "export-manifest.schema.json",
]

STATUS_TERMS = ["未开始", "PLAN_FROZEN", "ALGO_FROZEN", "尚未写应用"]


def run(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=repo,
        check=check,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


def git(repo: Path, *args: str) -> str:
    return run(repo, "git", *args).stdout


def git_bytes(repo: Path, ref: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=repo,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def line_count(data: bytes) -> int:
    return len(data.decode("utf-8").splitlines())


def first_diff_line(left: bytes, right: bytes) -> str:
    left_lines = left.decode("utf-8").splitlines()
    right_lines = right.decode("utf-8").splitlines()
    for number, (a, b) in enumerate(zip(left_lines, right_lines), start=1):
        if a != b:
            return f"line {number}"
    if len(left_lines) != len(right_lines):
        return f"line {min(len(left_lines), len(right_lines)) + 1} (length differs)"
    return "-"


def refs_in(value: Any) -> Iterable[str]:
    if isinstance(value, dict):
        ref = value.get("$ref")
        if isinstance(ref, str):
            yield ref
        for child in value.values():
            yield from refs_in(child)
    elif isinstance(value, list):
        for child in value:
            yield from refs_in(child)


def section(title: str) -> None:
    print()
    print(f"=== {title} ===")


def docs_inventory(repo: Path) -> None:
    section("1. docs/ file inventory")
    inventories: dict[str, list[str]] = {}
    for label, ref in REFS.items():
        commit = git(repo, "rev-parse", "--short=12", ref).strip()
        output = git(repo, "ls-tree", "-r", "--name-only", ref, "--", "docs")
        files = [line for line in output.splitlines() if line]
        inventories[label] = files
        print(f"{label}\tref={ref}\tcommit={commit}\tfiles={len(files)}")
        for path in files:
            print(f"  {path}")

    current = set(inventories["current"])
    print("delta versus current (other - current / current - other)")
    for label in ("product-lock", "goal1", "main"):
        other = set(inventories[label])
        added = sorted(other - current)
        removed = sorted(current - other)
        print(f"{label}\tother-only={len(added)}\tcurrent-only={len(removed)}")
        print(f"  other-only: {', '.join(added) if added else '-'}")
        print(f"  current-only: {', '.join(removed) if removed else '-'}")


def dual_source(repo: Path) -> None:
    section("2. dual-source hashes and comparisons")
    context_dir = repo / ".agent_workspace/context/plan"
    context_files = sorted(path for path in context_dir.iterdir() if path.is_file())
    print("context_path\tdocs_path\tcontext_sha256\tdocs_sha256\texact\tlines(context/docs)\tfirst_diff")
    for context_path in context_files:
        if context_path.suffix == ".json":
            docs_path = repo / "docs/schemas" / context_path.name
        else:
            docs_path = repo / "docs" / context_path.name
        context_data = context_path.read_bytes()
        if not docs_path.exists():
            print(
                f"{context_path.relative_to(repo)}\tMISSING\t{sha256(context_data)}\t-\tNO\t"
                f"{line_count(context_data)}/-\tmissing docs counterpart"
            )
            continue
        docs_data = docs_path.read_bytes()
        print(
            f"{context_path.relative_to(repo)}\t{docs_path.relative_to(repo)}\t"
            f"{sha256(context_data)}\t{sha256(docs_data)}\t"
            f"{'YES' if context_data == docs_data else 'NO'}\t"
            f"{line_count(context_data)}/{line_count(docs_data)}\t"
            f"{first_diff_line(context_data, docs_data)}"
        )

    product_locks = sorted(
        path
        for path in repo.rglob("PRODUCT_LOCK.md")
        if ".git" not in path.parts
    )
    print("all workspace PRODUCT_LOCK.md copies")
    for path in product_locks:
        data = path.read_bytes()
        print(f"  {path.relative_to(repo)}\tsha256={sha256(data)}\tbytes={len(data)}")


def schema_refs(repo: Path) -> None:
    section("3. nine schemas: references to _defs")
    print("schema\tbranch\texternal_defs_refs\tlocal_defs_refs\tother_refs\tlocal_defs_declared\tclassification")
    for schema_name in NINE_SCHEMAS:
        path = f"docs/schemas/{schema_name}"
        for label in ("current", "goal1"):
            raw = git_bytes(repo, REFS[label], path)
            parsed = json.loads(raw)
            refs = list(refs_in(parsed))
            external = [ref for ref in refs if "_defs.schema.json#" in ref]
            local = [ref for ref in refs if ref.startswith("#/$defs/")]
            other = [ref for ref in refs if ref not in external and ref not in local]
            has_defs = isinstance(parsed.get("$defs"), dict)
            if external:
                classification = "REFS_EXTERNAL_DEFS"
            elif local and not has_defs:
                classification = "DANGLING_LOCAL_DEFS"
            elif local:
                classification = "REFS_OWN_DEFS"
            else:
                classification = "NO_DEFS_REF"
            print(
                f"{schema_name}\t{label}\t{len(external)}\t{len(local)}\t{len(other)}\t"
                f"{'yes' if has_defs else 'no'}\t{classification}"
            )
            for ref in refs:
                print(f"  $ref={ref}")


def status_scan(repo: Path) -> None:
    section("4. STATUS falsehood term scan")
    for label, ref in REFS.items():
        path = "docs/STATUS.md"
        try:
            text = git_bytes(repo, ref, path).decode("utf-8")
        except subprocess.CalledProcessError:
            print(f"{label}\t{ref}\t{path}=MISSING")
            continue
        print(f"{label}\t{ref}\t{path}")
        any_match = False
        for number, line in enumerate(text.splitlines(), start=1):
            matched = [term for term in STATUS_TERMS if term in line]
            if matched:
                any_match = True
                print(f"  {number}: [{','.join(matched)}] {line}")
        if not any_match:
            print("  no matches")


def cargo_workspace(repo: Path) -> None:
    section("5. Cargo workspace metadata")
    result = run(
        repo,
        "cargo",
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
        check=False,
    )
    print(f"command=cargo metadata --no-deps --format-version 1\texit={result.returncode}")
    if result.returncode != 0:
        print(result.stdout.rstrip())
        return
    metadata = json.loads(result.stdout)
    packages = {package["id"]: package for package in metadata["packages"]}
    members = [packages[package_id] for package_id in metadata["workspace_members"]]
    print(f"workspace_members={len(members)}")
    for package in members:
        manifest = Path(package["manifest_path"]).relative_to(repo)
        print(
            f"  name={package['name']}\tversion={package['version']}\t"
            f"manifest={manifest}\trust_version={package.get('rust_version')}"
        )
    only_algo = bool(members) and all(package["name"].startswith("soul-algo-") for package in members)
    print(f"only_soul_algo_named_crates={'YES' if only_algo else 'NO'}")


def json_parse(repo: Path) -> None:
    section("6. JSON parse: docs/schemas/*.json")
    paths = sorted((repo / "docs/schemas").glob("*.json"))
    failures = 0
    for path in paths:
        try:
            json.loads(path.read_text(encoding="utf-8"))
            print(f"OK\t{path.relative_to(repo)}")
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            failures += 1
            print(f"FAIL\t{path.relative_to(repo)}\t{error}")
    print(f"summary\tfiles={len(paths)}\tpassed={len(paths) - failures}\tfailed={failures}")


def main() -> int:
    repo = Path(sys.argv[1] if len(sys.argv) > 1 else "/workspace").resolve()
    print("Soul plan-polish baseline probes")
    print(f"repo={repo}")
    print(f"branch={git(repo, 'branch', '--show-current').strip()}")
    print(f"head={git(repo, 'rev-parse', 'HEAD').strip()}")
    docs_inventory(repo)
    dual_source(repo)
    schema_refs(repo)
    status_scan(repo)
    cargo_workspace(repo)
    json_parse(repo)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
