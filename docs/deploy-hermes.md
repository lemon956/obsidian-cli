# Hermes 端部署与使用

本文档面向运行 Hermes、Agent 或自动化脚本的 Linux 调用端。

## 1. 构建单文件二进制

在源码目录执行：

```bash
cargo build --release
```

安装到系统 PATH：

```bash
sudo install -m 0755 target/release/webdav-cli /usr/local/bin/webdav-cli
webdav-cli --help
```

生产机不需要保留 Rust 工具链；可以在构建机产出 `target/release/webdav-cli` 后复制到 Hermes 机器。

## 2. 配置 WebDAV 连接

初始化配置：

```bash
webdav-cli init
```

交互式输入顺序：

```text
WebDAV URL
Username
Password env name
Default write directory
```

也可以非交互初始化：

```bash
webdav-cli init \
  --url "https://example.com/obsidian-webdav/" \
  --username "hermes" \
  --password-env "OBSIDIAN_WEBDAV_PASSWORD" \
  --write-dir "Inbox/Hermes"
```

如果 Obsidian 客户端已经直接使用服务端的 `/webdav/`，Hermes 端的 `webdav-cli` 建议配置为 `/obsidian-webdav/`。这样可以让 Obsidian 保持完整 WebDAV 功能，同时让 Agent 只经过受限入口。

设置密码环境变量：

```bash
export OBSIDIAN_WEBDAV_PASSWORD='your-webdav-password'
```

如果 Hermes 通过 systemd 运行，建议写入 service 的环境文件：

```ini
# /etc/hermes/webdav-cli.env
OBSIDIAN_WEBDAV_PASSWORD=your-webdav-password
```

并在 service 中引用：

```ini
EnvironmentFile=/etc/hermes/webdav-cli.env
```

不要把 WebDAV 密码写入 `config.yaml` 或仓库。

## 3. 验证连接

```bash
webdav-cli doctor
```

`doctor` 会检查配置、WebDAV 连接、根目录读取、默认写入目录存在、默认写入目录可写、正式目录只读，以及服务端是否禁止 `DELETE`。

如果只想跳过写入探测：

```bash
webdav-cli doctor --no-write-test
```

期望看到类似输出：

```text
[OK] Config loaded
[OK] WebDAV connected
[OK] Root vault readable
[OK] Inbox/Hermes exists
[OK] Inbox/Hermes writable
[OK] Notes directory readonly
[OK] DELETE forbidden
```

## 4. Hermes 调用示例

写入一条普通笔记：

```bash
webdav-cli new \
  --title "Hermes 与 Obsidian WebDAV 权限设计" \
  --source telegram \
  --tag hermes \
  --tag obsidian \
  --tag webdav \
  --body "Hermes 需要读取整个 vault，但只能向 Inbox/Hermes 写入。"
```

写入排障模板：

```bash
webdav-cli new \
  --title "Hermes Gateway Debug" \
  --template troubleshooting \
  --source log \
  --tag hermes \
  --tag debug \
  --body "systemd 日志显示 gateway 停止时仍有其他 hermes 进程存在。"
```

从管道写入：

```bash
journalctl -u hermes-gateway -n 100 --no-pager \
  | webdav-cli new \
      --title "Hermes Gateway systemd 日志" \
      --template troubleshooting \
      --source log \
      --tag hermes \
      --tag systemd
```

如果没有 `--body`，`webdav-cli new` 会从 stdin 读取正文，适合 Hermes shell tool 和日志管道调用。

搜索已有知识：

```bash
webdav-cli search "Telegram gateway"
webdav-cli search "sshd dead" --dir Troubleshooting
```

读取笔记正文：

```bash
webdav-cli cat Notes/Hermes.md --body
```

## 5. 推荐 Hermes Prompt 约束

可加入 Hermes 的系统提示词或工具说明：

```text
当用户说“记到 Obsidian”、“保存到知识库”、“写入个人知识库”时：

1. 使用 webdav-cli new 创建笔记。
2. 默认写入 Inbox/Hermes/。
3. 不要尝试写入 Notes/、Projects/、Index/ 等正式目录。
4. 不要删除、移动、覆盖任何笔记。
5. 标题应简洁明确。
6. 正文使用 Markdown。
7. 技术排查类内容使用 troubleshooting 模板。
8. 标签不少于 2 个，不超过 6 个。
9. 如果内容来自 Telegram，source 设置为 telegram。
10. 如果内容来自 shell/log，source 设置为 shell 或 log。
```
