# Command Recipes

## Setup Checks

```bash
webdav-cli --help
webdav-cli doctor
webdav-cli doctor --no-write-test
```

Use an explicit config path when needed:

```bash
webdav-cli --config /path/to/config.yaml doctor
```

Set the password in the agent runtime:

```bash
export OBSIDIAN_WEBDAV_PASSWORD='your-webdav-password'
```

## Read

```bash
webdav-cli ls
webdav-cli ls Notes
webdav-cli cat Notes/Example.md
webdav-cli cat Notes/Example.md --body
webdav-cli cat Notes/Example.md --frontmatter
```

## Search

```bash
webdav-cli search "Hermes gateway"
webdav-cli search "systemd" --dir Troubleshooting
webdav-cli search "keyword" --json
```

Search before creating a new note if the user is asking whether something already exists.

## Create Notes

Short body:

```bash
webdav-cli new \
  --title "Title" \
  --source hermes \
  --tag inbox \
  --tag webdav \
  --body "Markdown body"
```

Long body from stdin:

```bash
printf '%s\n' "$NOTE_BODY" | webdav-cli new \
  --title "Title" \
  --source codex \
  --tag inbox \
  --tag note
```

Troubleshooting note:

```bash
webdav-cli new \
  --title "Service startup failure" \
  --template troubleshooting \
  --source shell \
  --tag debug \
  --tag systemd \
  --body "Observed behavior, logs, hypothesis, next checks."
```

## Create Allowed Inbox Directories

```bash
webdav-cli mkdir Inbox/Hermes/topic
webdav-cli mkdir -p Inbox/Hermes/topic/deep
```

Do not create directories outside the configured write allowlist.

## Controlled Inbox Operations

All paths must stay inside `allow_write_dirs`, normally `Inbox/Hermes`.

```bash
webdav-cli copy Inbox/Hermes/a.md Inbox/Hermes/b.md
webdav-cli copy Inbox/Hermes/a.md Inbox/Hermes/b.md --depth 0 --overwrite
webdav-cli proppatch Inbox/Hermes/a.md --xml '<propertyupdate />'
webdav-cli lock Inbox/Hermes/a.md --owner hermes --timeout 120
webdav-cli unlock Inbox/Hermes/a.md --token 'opaquelocktoken:123'
```

Only run these when the config explicitly enables them:

```bash
webdav-cli delete Inbox/Hermes/old.md
webdav-cli move Inbox/Hermes/old.md Inbox/Hermes/new.md --overwrite
```

## JSON Mode

Use JSON only for machine parsing:

```bash
webdav-cli ls --json
webdav-cli search "query" --json
webdav-cli new --title "Title" --body "Body" --json
webdav-cli delete Inbox/Hermes/old.md --json
webdav-cli move Inbox/Hermes/old.md Inbox/Hermes/new.md --json
webdav-cli copy Inbox/Hermes/a.md Inbox/Hermes/b.md --json
webdav-cli proppatch Inbox/Hermes/a.md --xml '<propertyupdate />' --json
webdav-cli lock Inbox/Hermes/a.md --json
webdav-cli unlock Inbox/Hermes/a.md --token 'opaquelocktoken:123' --json
webdav-cli doctor --json
```
