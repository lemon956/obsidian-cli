# Agent Adapters

This skill is designed around a shell command, so every agent should preserve the same security model: call `webdav-cli`; do not write to WebDAV directly.

## Shared Runtime Contract

Install:

```bash
cargo build --release
sudo install -m 0755 target/release/webdav-cli /usr/local/bin/webdav-cli
```

Configure:

```bash
webdav-cli init \
  --url "https://example.com/obsidian-webdav/" \
  --username "hermes" \
  --password-env "OBSIDIAN_WEBDAV_PASSWORD" \
  --write-dir "Inbox/Hermes"
```

Runtime environment:

```bash
export OBSIDIAN_WEBDAV_PASSWORD='your-webdav-password'
```

The restricted server route must grant full HTTP/WebDAV methods under `Inbox/Hermes`: `GET`, `HEAD`, `OPTIONS`, `PROPFIND`, `PUT`, `MKCOL`, `DELETE`, `MOVE`, `COPY`, `PROPPATCH`, `LOCK`, and `UNLOCK`. Formal Vault directories should remain read-only through the restricted endpoint.

If the WebDAV root contains a vault folder, include that folder in `base_url`, for example:

```yaml
webdav:
  base_url: https://example.com/obsidian-webdav/lemon/
```

## Hermes

Put the core rules from `SKILL.md` into Hermes tool instructions or a Hermes skill:

```text
Use webdav-cli for Obsidian. Read/search anywhere. The /obsidian-webdav/Inbox/Hermes/ route has full HTTP/WebDAV permissions; routine note creation should use `webdav-cli new` or `webdav-cli mkdir` under Inbox/Hermes. Do not modify formal directories or call raw WebDAV writes when webdav-cli can perform the operation. Use OBSIDIAN_WEBDAV_PASSWORD from the runtime environment.
```

For systemd-based Hermes, store the password in an environment file and reference it from the service:

```ini
EnvironmentFile=/etc/hermes/webdav-cli.env
```

## OpenClaw

Expose `webdav-cli` as a shell tool command and attach the safety prompt:

```text
When using Obsidian, invoke webdav-cli. Never use curl/rclone/direct WebDAV for writes. Save new knowledge with `webdav-cli new`; search with `webdav-cli search`; read with `webdav-cli cat`.
```

If OpenClaw supports tool allowlists, allow `webdav-cli` and deny destructive WebDAV commands.

## Codex

Copy this folder to a Codex skill path when you want automatic discovery:

```bash
mkdir -p "${CODEX_HOME:-$HOME/.codex}/skills"
cp -R skills/obsidian-webdav "${CODEX_HOME:-$HOME/.codex}/skills/"
```

Then requests such as "save this to Obsidian", "search my Obsidian notes", or "read this Vault note" should trigger the skill.

## Claude

Claude Projects or other Claude-based agents can use this as project instructions. Paste the `Overview`, `Safety Rules`, and `Task Workflow` sections into the project knowledge/instructions, and make sure the runtime exposes a shell tool that can call `webdav-cli`.

Recommended Claude instruction:

```text
For Obsidian tasks, use webdav-cli only. You may list, read, and search the Vault. The Inbox/Hermes route has full HTTP/WebDAV permissions, but normal note creation should still go through webdav-cli new. Never modify formal directories or write outside the configured allow_write_dirs.
```

## Generic Agent Prompt

Use this when an agent does not support formal skills:

```text
You have access to Obsidian through webdav-cli. Use it for Vault access. Commands: doctor, ls, cat, search, new, mkdir. Read/search anywhere. The Inbox/Hermes route has full HTTP/WebDAV permissions; routine writes should still be new notes under Inbox/Hermes. Do not modify formal directories or use raw WebDAV writes when webdav-cli can perform the operation. Use Markdown, concise titles, useful tags, and source metadata.
```
