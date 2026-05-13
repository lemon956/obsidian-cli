---
name: obsidian-webdav
description: Use webdav-cli to read, search, create notes, and perform controlled WebDAV operations in an Obsidian Vault. Trigger when an agent such as Hermes, OpenClaw, Codex, or Claude needs to save knowledge, search or read notes, validate WebDAV connectivity, or operate on the configured Agent inbox such as Inbox/Hermes.
---

# Obsidian WebDAV

## Overview

Use `webdav-cli` as the normal interface to the Obsidian Vault. Treat the Vault as broadly readable, with full HTTP/WebDAV method permissions only under the configured Agent inbox, normally `Inbox/Hermes`. Routine note creation goes through `new` or `mkdir`; controlled inbox maintenance uses `delete`, `move`, `copy`, `proppatch`, `lock`, and `unlock`.

This skill is portable across Hermes, OpenClaw, Codex, Claude, and similar shell-capable agents. If an agent has a shell tool, call `webdav-cli` directly; if it only supports prompt/tool instructions, embed the rules from this file.

## Prerequisites

Before using the Vault, verify:

- `webdav-cli` is installed and available on `PATH`, or the caller knows its absolute path.
- The config path is known. Default: `~/.config/webdav-cli/config.yaml`.
- The password env var named by config, usually `OBSIDIAN_WEBDAV_PASSWORD`, is set in the agent runtime.
- `webdav.base_url` points at the restricted CLI endpoint, commonly `/obsidian-webdav/`, not the full Obsidian client endpoint unless the user explicitly intends that.
- `behavior.allow_write_dirs` contains the only CLI write targets, usually `Inbox/Hermes`.
- The server route grants `Inbox/Hermes` full HTTP/WebDAV methods: `GET`, `HEAD`, `OPTIONS`, `PROPFIND`, `PUT`, `MKCOL`, `DELETE`, `MOVE`, `COPY`, `PROPPATCH`, `LOCK`, and `UNLOCK`.

Run this before first use or after server changes:

```bash
webdav-cli doctor
```

Use `webdav-cli --config /path/to/config.yaml ...` when the config is not in the default location.

## Safety Rules

- Never call raw `curl`, `rclone`, filesystem commands, or direct WebDAV writes to modify the Vault when `webdav-cli` can perform the operation.
- Never delete, move, rename, copy, overwrite, or sync formal Vault content. The server route may grant full methods under `Inbox/Hermes`, but controlled operations must still use `webdav-cli`.
- Never write to `Notes`, `Projects`, `Daily`, `Index`, `Sources`, `Troubleshooting`, `.obsidian`, or `Attachments`.
- Use `delete` only when `behavior.allow_delete` is true and the user explicitly wants removal inside the Agent inbox.
- Use `move` only when `behavior.allow_move` is true and both source and destination are inside the Agent inbox.
- Use `copy`, `proppatch`, `lock`, and `unlock` only inside `allow_write_dirs`.
- Do not bypass `allow_write_dirs`; if a requested destination is outside the Agent inbox, create a note in the inbox explaining the desired destination for human review.
- Do not store WebDAV passwords in prompts, notes, config files, logs, or command arguments. Use the configured environment variable.
- Prefer `--json` only when the caller will parse structured output; otherwise use plain output for human-readable interactions.

## Task Workflow

1. For "save this", "remember this", "write to Obsidian", or "add to knowledge base", create a new inbox note:

```bash
webdav-cli new \
  --title "Short specific title" \
  --source agent \
  --tag inbox \
  --tag agent \
  --body "Markdown body"
```

2. For logs or long generated content, pipe stdin instead of putting large text in arguments:

```bash
printf '%s\n' "$NOTE_BODY" | webdav-cli new \
  --title "Short specific title" \
  --template troubleshooting \
  --source shell \
  --tag debug
```

3. For finding context, search before creating duplicates:

```bash
webdav-cli search "query terms"
webdav-cli search "query terms" --dir Notes
```

4. For reading a known note:

```bash
webdav-cli cat Notes/Example.md --body
```

5. For browsing:

```bash
webdav-cli ls
webdav-cli ls Inbox/Hermes
```

6. For creating subdirectories, only create them under the allowed write directory:

```bash
webdav-cli mkdir Inbox/Hermes/topic
webdav-cli mkdir -p Inbox/Hermes/topic/deep
```

7. For controlled inbox maintenance, stay inside `allow_write_dirs`:

```bash
webdav-cli copy Inbox/Hermes/a.md Inbox/Hermes/b.md
webdav-cli lock Inbox/Hermes/a.md --owner hermes
webdav-cli unlock Inbox/Hermes/a.md --token 'opaquelocktoken:123'
```

Only use these after confirming config gates and user intent:

```bash
webdav-cli delete Inbox/Hermes/old.md
webdav-cli move Inbox/Hermes/old.md Inbox/Hermes/new.md
```

## Note Quality

When creating notes:

- Use a concise, searchable title.
- Write Markdown with enough context for future retrieval.
- Include factual source context, such as `telegram`, `shell`, `log`, `codex`, `claude`, `openclaw`, or `hermes`.
- Use 2-6 meaningful tags.
- Use `--template troubleshooting` for failures, logs, diagnostics, deployment problems, and command output analysis.
- If the user asks to update an existing formal note, create a new inbox note that describes the requested update instead of editing the formal note.

## Failure Handling

- `missing_password_env`: set the environment variable named in `config.webdav.password_env`.
- `401 Unauthorized`: verify WebDAV username/password and that Nginx forwards `Authorization`.
- `404 Not Found: PROPFIND`: verify `webdav.base_url` matches the actual WebDAV root. If root listing shows a vault folder such as `/lemon/`, set `base_url` to `/obsidian-webdav/lemon/` or remap Nginx.
- `write_not_allowed`: the requested path is outside `allow_write_dirs`; write to the inbox instead.
- `delete requires behavior.allow_delete = true`: enable the config only if inbox deletion is intended.
- `move requires behavior.allow_move = true`: enable the config only if inbox moves are intended.
- `unlock requires a non-empty token`: pass the lock token from the `lock` response.
- `lock timeout must be "infinite" or a positive integer`: use `--timeout infinite` or seconds such as `--timeout 120`.
- `<dir> lacks full HTTP permissions; missing: ...`: update the server route so `Inbox/Hermes` allows `GET`, `HEAD`, `OPTIONS`, `PROPFIND`, `PUT`, `MKCOL`, `DELETE`, `MOVE`, `COPY`, `PROPPATCH`, `LOCK`, and `UNLOCK`.
- `<dir> is not readonly`: a formal Vault directory exposes write or destructive methods; fix the restricted endpoint before using it for writes.

## References

- Read `references/command-recipes.md` for concise command recipes.
- Read `references/agent-adapters.md` when installing or embedding this skill in Hermes, OpenClaw, Codex, Claude, or another agent.
