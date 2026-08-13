# Security Policy

## Supported versions

LLM Neurosurgeon is currently pre-release software. Security fixes are applied to the latest commit on the `main` branch. Published releases will receive an explicit support table once the first release candidate is available.

## Reporting a vulnerability

Please do not open a public GitHub issue for a suspected vulnerability.

Use GitHub's private vulnerability reporting for this repository:

https://github.com/earnerbaymalay/llm-neuro-surgeon/security/advisories/new

Include:

- the affected component and version or commit;
- reproduction steps or a minimal proof of concept;
- the security impact;
- affected operating systems and configurations;
- any suggested mitigation, if known.

Do not include real API keys, credentials, private configuration files, or personal data in the report.

## Response process

The maintainer will acknowledge a complete report, validate its impact, prepare a fix, and coordinate disclosure through a GitHub Security Advisory when appropriate. Exact response times are not guaranteed while the project is pre-release.

## Scope

Security-sensitive areas include filesystem projection, symlink and path handling, Git rollback, imported marketplace content, MCP server configuration, subprocess execution, updater behavior, and OS keychain integration.
