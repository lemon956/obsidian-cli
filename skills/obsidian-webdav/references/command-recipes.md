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

## JSON Mode

Use JSON only for machine parsing:

```bash
webdav-cli ls --json
webdav-cli search "query" --json
webdav-cli new --title "Title" --body "Body" --json
webdav-cli doctor --json
```
