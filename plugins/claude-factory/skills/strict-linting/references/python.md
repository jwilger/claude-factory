# Strict Linting — Python

## Ruff — linter and formatter

Ruff replaces flake8 + isort + black in one fast binary. Use the strict preset:

```toml
# pyproject.toml
[tool.ruff]
target-version = "py312"
line-length = 100

[tool.ruff.lint]
select = [
    "E",   # pycodestyle errors
    "W",   # pycodestyle warnings
    "F",   # pyflakes
    "I",   # isort
    "N",   # pep8-naming
    "UP",  # pyupgrade
    "B",   # flake8-bugbear
    "A",   # flake8-builtins
    "C4",  # flake8-comprehensions
    "DTZ", # flake8-datetimez (timezone-aware datetimes always)
    "T20", # flake8-print (no print() in production code)
    "RET", # flake8-return
    "SIM", # flake8-simplify
    "ARG", # flake8-unused-arguments
    "PTH", # flake8-use-pathlib
    "ERA", # eradicate (no commented-out code)
    "PL",  # pylint
    "PERF",# perflint
    "RUF", # ruff-specific
]
ignore = []  # start with nothing ignored; add with justification

[tool.ruff.lint.per-file-ignores]
"tests/**" = ["T20", "ARG"]  # print() in tests, unused fixtures are fine

[tool.ruff.format]
quote-style = "double"
indent-style = "space"
```

## Mypy — strict type checking

```toml
# pyproject.toml
[tool.mypy]
python_version = "3.12"
strict = true
warn_return_any = true
warn_unused_configs = true
disallow_any_explicit = true       # no Any in annotations
disallow_any_generics = true       # no List without type param
disallow_subclassing_any = true
disallow_untyped_decorators = true
no_implicit_reexport = true
extra_checks = true

# Per-module overrides (only for third-party stubs that don't exist yet)
# [[tool.mypy.overrides]]
# module = "some_untyped_library.*"
# ignore_missing_imports = true
```

`strict = true` enables: `disallow_untyped_defs`, `disallow_incomplete_defs`, `check_untyped_defs`, `disallow_untyped_decorators`, `warn_redundant_casts`, `warn_unused_ignores`, `no_implicit_optional`, `strict_equality`. `disallow_any_explicit` goes beyond `strict` — add it separately.

## Pre-commit integration

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.6.0
    hooks:
      - id: ruff
        args: [--fix]
      - id: ruff-format
  - repo: https://github.com/pre-commit/mirrors-mypy
    rev: v1.11.0
    hooks:
      - id: mypy
        additional_dependencies: [pydantic>=2, types-...]
```

## When to relax

`# type: ignore[<specific-code>]` — never bare `# type: ignore`. Add a `# Reason:` comment. Common legitimate cases: third-party libraries without stubs where `ignore_missing_imports` is not sufficient; dynamic attribute access on plugin/extension boundaries where the type cannot be statically known.
