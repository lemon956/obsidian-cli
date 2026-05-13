# webdav-cli

`webdav-cli` 是一个用 Rust 编写的 Obsidian WebDAV 命令行工具，面向 Hermes、Agent 和自动化脚本使用。

它的定位是安全访问层：

- 可以读取整个 Obsidian Vault。
- 可以列目录、读笔记、搜索 Markdown。
- 默认只能在 `Inbox/Hermes` 范围内写入和执行受控 WebDAV 管理操作。
- 服务端受限入口需要给 `Inbox/Hermes` 完整 HTTP/WebDAV 方法权限。
- 默认不覆盖、不删除、不移动、不重命名正式笔记。
- 生成带 YAML frontmatter 的 Obsidian Markdown。

## 安装

```bash
cargo build --release
sudo install -m 0755 target/release/webdav-cli /usr/local/bin/webdav-cli
```

也可以通过 GitHub Actions 生成可执行文件：

- 手动触发：进入 GitHub 仓库的 `Actions` -> `Build webdav-cli binaries` -> `Run workflow`。
- 分支触发：推送到 `main` 会构建并上传 workflow artifact。
- tag 触发：新建并推送 `v*` tag，例如 `git tag v0.1.3 && git push origin v0.1.3`，会构建并上传到对应 GitHub Release。workflow 需要先存在于默认分支 `main`。
- 构建完成后，在 workflow run 的 Artifacts 下载 `webdav-cli-linux-x86_64`、`webdav-cli-macos` 或 `webdav-cli-windows-x86_64`。

Linux 下载后安装：

```bash
tar -xzf webdav-cli-linux-x86_64.tar.gz
sudo install -m 0755 webdav-cli /usr/local/bin/webdav-cli
```

初始化配置：

```bash
webdav-cli init
```

也可以非交互初始化：

```bash
webdav-cli init \
  --url "https://example.com/obsidian-webdav/" \
  --username "hermes" \
  --password-env "OBSIDIAN_WEBDAV_PASSWORD" \
  --write-dir "Inbox/Hermes"

export OBSIDIAN_WEBDAV_PASSWORD='your-password'
```

默认配置文件路径：

```text
~/.config/webdav-cli/config.yaml
```

如果 Obsidian 客户端本身已经在使用 `/webdav/`，不要把这个入口改成只读或半只读。建议在 Nginx 中保留 `/webdav/` 给 Obsidian 正常同步，再新增 `/obsidian-webdav/` 作为 `webdav-cli` 的受限入口。

## 常用命令

```bash
webdav-cli doctor
webdav-cli ls
webdav-cli ls Notes
webdav-cli cat Notes/Hermes.md
webdav-cli cat Notes/Hermes.md --body
webdav-cli search "Hermes gateway"
webdav-cli search "systemd" --dir Troubleshooting
```

创建笔记：

```bash
webdav-cli new \
  --title "Hermes Gateway Debug" \
  --template troubleshooting \
  --source telegram \
  --tag hermes \
  --tag debug \
  --body "Hermes 流式输出在某个字卡住。"
```

从 stdin 创建笔记：

```bash
journalctl -u hermes-gateway -n 100 --no-pager \
  | webdav-cli new \
      --title "Hermes Gateway systemd 日志" \
      --template troubleshooting \
      --source log \
      --tag hermes \
      --tag systemd
```

创建允许写入目录下的子目录：

```bash
webdav-cli mkdir Inbox/Hermes/debug
webdav-cli mkdir -p Inbox/Hermes/debug/deep
```

`mkdir -p` 只会从允许写入目录内部开始逐级创建，不会尝试创建或修改 `Inbox` 这类上级目录。

受控 WebDAV 方法命令仅允许操作 `allow_write_dirs` 内路径。`delete` 需要 `behavior.allow_delete: true`，`move` 需要 `behavior.allow_move: true`：

```bash
webdav-cli delete Inbox/Hermes/old.md
webdav-cli move Inbox/Hermes/old.md Inbox/Hermes/new.md --overwrite
webdav-cli copy Inbox/Hermes/a.md Inbox/Hermes/b.md --depth 0
webdav-cli proppatch Inbox/Hermes/a.md --xml '<propertyupdate />'
webdav-cli lock Inbox/Hermes/a.md --owner hermes
webdav-cli unlock Inbox/Hermes/a.md --token 'opaquelocktoken:123'
```

`doctor` 会检查配置、WebDAV 连接、根目录读取、默认写入目录存在、`Inbox/Hermes` 完整 HTTP/WebDAV 方法权限、默认写入目录可写，以及正式目录只读。如需跳过写入探测：

```bash
webdav-cli doctor --no-write-test
```

## JSON 输出

支持 JSON 的命令：

```bash
webdav-cli ls --json
webdav-cli search "Hermes" --json
webdav-cli new --title "测试" --body "hello" --json
webdav-cli mkdir Inbox/Hermes/debug --json
webdav-cli delete Inbox/Hermes/old.md --json
webdav-cli move Inbox/Hermes/old.md Inbox/Hermes/new.md --json
webdav-cli copy Inbox/Hermes/a.md Inbox/Hermes/b.md --json
webdav-cli proppatch Inbox/Hermes/a.md --xml '<propertyupdate />' --json
webdav-cli lock Inbox/Hermes/a.md --json
webdav-cli unlock Inbox/Hermes/a.md --token 'opaquelocktoken:123' --json
webdav-cli doctor --json
```

错误 JSON 示例：

```json
{
  "ok": false,
  "error": "write_not_allowed",
  "message": "writing to Notes/test.md is not allowed; allowed write directories: Inbox/Hermes"
}
```

## 双端部署文档

- Hermes/Linux 调用端：[docs/deploy-hermes.md](docs/deploy-hermes.md)
- Obsidian Vault WebDAV/Nginx 服务端：[docs/deploy-webdav-nginx.md](docs/deploy-webdav-nginx.md)
- Agent skill：[skills/obsidian-webdav/SKILL.md](skills/obsidian-webdav/SKILL.md)

`webdav-cli` 不需要部署服务端进程。服务端需要运行已有 WebDAV 服务；Hermes 端只安装 CLI 二进制并通过 `/obsidian-webdav/` 受限入口访问 Vault。

## 安全原则

`webdav-cli` 不提供 `sync` 命令。所有写入、删除、移动、复制和属性/锁操作都会经过路径校验，只允许作用于 `allow_write_dirs`；默认配置只允许 `Inbox/Hermes`。`delete` 和 `move` 还需要分别显式开启 `allow_delete` 和 `allow_move`。
