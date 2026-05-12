# obsidian-cli

`obsidian-cli` 是一个面向 Hermes / Agent / 自动化脚本的 Obsidian WebDAV 命令行工具。

它的目标不是替代 Obsidian 客户端，而是提供一个安全、可控、适合 AI Agent 使用的接口，让 Hermes 可以：

1. 读取整个 Obsidian Vault
2. 搜索 Markdown 笔记内容
3. 向指定 Inbox 子目录写入新笔记
4. 禁止修改、删除、覆盖正式笔记
5. 以结构化 Markdown 方式沉淀个人知识库内容

---

## 背景

当前 Hermes 和 Obsidian 不在同一台服务器上。

Obsidian Vault 通过 WebDAV 暴露给 Hermes 使用，但权限模型需要满足：

```text
Hermes 可以读取整个 Obsidian Vault
Hermes 只能向指定子目录写入内容
Hermes 不能修改 Notes / Projects / Troubleshooting / Index 等正式目录
Hermes 不能删除任何笔记
Hermes 不能移动、重命名、覆盖正式笔记
```

因此需要一个 `obsidian-cli` 工具，作为 Hermes 与 Obsidian WebDAV 之间的安全访问层。

---

## 目标

### 核心目标

`obsidian-cli` 应该提供以下能力：

```text
读取整个 Vault
列出目录
读取指定笔记
搜索笔记内容
创建新笔记
向 Inbox/Hermes 写入内容
生成符合 Obsidian 习惯的 Markdown
生成 YAML frontmatter
避免覆盖已有文件
```

### 非目标

`obsidian-cli` 不应该做以下事情：

```text
不负责启动 Obsidian
不负责渲染 Obsidian 页面
不直接操作 Obsidian 插件
不默认修改正式目录中的笔记
不默认删除任何文件
不默认执行 rclone sync
不默认执行 destructive 操作
```

---

## 推荐架构

```text
Hermes
  ↓
obsidian-cli
  ↓
WebDAV
  ↓
Obsidian Vault
```

WebDAV 权限应由 Nginx 或 WebDAV 服务器控制：

```text
GET / HEAD / OPTIONS / PROPFIND 允许访问整个 Vault
PUT / MKCOL 仅允许访问 /Inbox/Hermes/
DELETE / MOVE / COPY / PROPPATCH 默认禁止
```

---

## Vault 目录约定

推荐 Vault 目录结构：

```text
ObsidianVault/
├── Inbox/
│   └── Hermes/
├── Daily/
├── Notes/
├── Projects/
├── Troubleshooting/
├── Sources/
├── Index/
├── Templates/
└── Attachments/
```

Hermes 默认只能写入：

```text
Inbox/Hermes/
```

其他目录只读。

---

## 配置文件

默认配置文件路径：

```text
~/.config/obsidian-cli/config.yaml
```

示例：

```yaml
webdav:
  base_url: "https://example.com/webdav/"
  username: "hermes"
  password_env: "OBSIDIAN_WEBDAV_PASSWORD"
  timeout: 30

vault:
  default_write_dir: "Inbox/Hermes"
  timezone: "Asia/Shanghai"
  filename_time_format: "2006-01-02-150405"
  default_tags:
    - hermes
    - inbox

behavior:
  allow_overwrite: false
  allow_delete: false
  allow_move: false
  allow_write_dirs:
    - "Inbox/Hermes"
  readonly_dirs:
    - "Notes"
    - "Projects"
    - "Troubleshooting"
    - "Index"
    - "Daily"
    - "Sources"

markdown:
  frontmatter: true
  heading_title: true
  add_created_time: true
  add_source: true
  default_source: "hermes"
```

密码不应该明文写入配置文件，推荐使用环境变量：

```bash
export OBSIDIAN_WEBDAV_PASSWORD='your-password'
```

---

## 命令设计

### 1. 初始化配置

```bash
obsidian-cli init
```

交互式生成配置文件：

```text
WebDAV URL:
Username:
Password env name:
Default write directory:
```

也支持非交互：

```bash
obsidian-cli init \
  --url "https://example.com/webdav/" \
  --username "hermes" \
  --password-env "OBSIDIAN_WEBDAV_PASSWORD" \
  --write-dir "Inbox/Hermes"
```

---

### 2. 测试连接

```bash
obsidian-cli doctor
```

检查内容：

```text
配置文件是否存在
WebDAV 是否可连接
账号是否可认证
根目录 PROPFIND 是否成功
默认写入目录是否存在
默认写入目录是否可写
非写入目录是否不可写
DELETE 是否被禁止
```

期望输出：

```text
[OK] Config loaded
[OK] WebDAV connected
[OK] Root vault readable
[OK] Inbox/Hermes writable
[OK] Notes directory readonly
[OK] DELETE forbidden
```

---

### 3. 列出目录

```bash
obsidian-cli ls
```

等价于列出 Vault 根目录。

```bash
obsidian-cli ls Notes
obsidian-cli ls Inbox/Hermes
```

输出示例：

```text
Daily/
Inbox/
Notes/
Projects/
Troubleshooting/
Index/
```

支持 JSON：

```bash
obsidian-cli ls Notes --json
```

---

### 4. 读取笔记

```bash
obsidian-cli cat Notes/Hermes.md
```

输出 Markdown 原文。

支持只输出 frontmatter：

```bash
obsidian-cli cat Notes/Hermes.md --frontmatter
```

支持只输出正文：

```bash
obsidian-cli cat Notes/Hermes.md --body
```

---

### 5. 搜索笔记

```bash
obsidian-cli search "Hermes gateway"
```

搜索整个 Vault 中的 Markdown 文件。

默认行为：

```text
只搜索 .md 文件
忽略 .obsidian/
忽略 Attachments/
返回文件路径、行号、匹配片段
```

输出示例：

```text
Notes/Hermes.md:24: Hermes gateway 用于连接 Telegram
Troubleshooting/Hermes/stream-stuck.md:12: gateway 在流式输出时卡住
```

支持限制目录：

```bash
obsidian-cli search "Telegram" --dir Notes
obsidian-cli search "systemd" --dir Troubleshooting
```

支持 JSON：

```bash
obsidian-cli search "Hermes" --json
```

---

### 6. 创建新笔记

```bash
obsidian-cli new \
  --title "Hermes Telegram 流式卡顿排查" \
  --body "这里是正文内容"
```

默认写入：

```text
Inbox/Hermes/
```

生成文件名：

```text
Inbox/Hermes/2026-04-24-153012-hermes-telegram-流式卡顿排查.md
```

生成 Markdown：

```markdown
---
title: "Hermes Telegram 流式卡顿排查"
created: "2026-04-24 15:30:12"
source: "hermes"
status: "inbox"
tags:
  - hermes
  - inbox
---

# Hermes Telegram 流式卡顿排查

这里是正文内容
```

---

### 7. 从 stdin 创建笔记

适合 Hermes 或 shell 管道调用：

```bash
echo "这里是正文内容" | obsidian-cli new --title "测试笔记"
```

或者：

```bash
cat /tmp/hermes-summary.md | obsidian-cli new --title "Hermes 总结"
```

---

### 8. 指定标签

```bash
obsidian-cli new \
  --title "GreenCloud SSH 问题" \
  --tag vps \
  --tag ssh \
  --tag troubleshooting \
  --body "sshd 启动后立刻 dead"
```

生成：

```yaml
tags:
  - hermes
  - inbox
  - vps
  - ssh
  - troubleshooting
```

---

### 9. 指定来源

```bash
obsidian-cli new \
  --title "Telegram 记录" \
  --source telegram \
  --body "用户从 Telegram 发来的内容"
```

---

### 10. 使用模板创建笔记

```bash
obsidian-cli new \
  --title "Hermes Gateway Debug" \
  --template troubleshooting \
  --body "原始日志内容"
```

内置模板：

```text
basic
daily
troubleshooting
project
source
meeting
```

`troubleshooting` 模板示例：

```markdown
---
title: "{{title}}"
created: "{{created}}"
source: "{{source}}"
status: "inbox"
tags:
  - hermes
  - inbox
  - troubleshooting
---

# {{title}}

## 问题

{{body}}

## 环境

## 现象

## 日志

## 初步判断

## 解决方案

## 验证命令

## 相关笔记
```

---

## 安全策略

### 默认禁止覆盖

如果目标文件已存在，默认不覆盖：

```text
Error: file already exists
```

可以自动追加时间戳：

```bash
obsidian-cli new --title "测试" --unique
```

不建议提供默认覆盖能力。

如果必须支持覆盖，需要显式参数：

```bash
obsidian-cli new --title "测试" --overwrite
```

并且 `config.yaml` 中必须允许：

```yaml
behavior:
  allow_overwrite: true
```

---

### 默认禁止删除

不提供普通删除命令。

如果未来需要删除，只能删除 `Inbox/Hermes/` 下的文件，并要求显式开启：

```yaml
behavior:
  allow_delete: false
```

默认不实现：

```bash
obsidian-cli delete
```

---

### 默认禁止移动

不提供默认移动正式笔记的能力。

原因：

```text
MOVE 可能破坏 Obsidian 双链
MOVE 可能导致笔记路径变化
MOVE 可能被 Agent 误用
```

---

## 路径限制

所有写入操作必须经过路径校验。

允许写入：

```text
Inbox/Hermes/
```

禁止写入：

```text
../
/absolute/path
.obsidian/
Notes/
Projects/
Troubleshooting/
Index/
Daily/
Sources/
Attachments/
```

路径校验规则：

```text
清理路径中的 ..
清理重复 /
禁止绝对路径
禁止 URL 编码绕过
禁止写入 allow_write_dirs 之外的目录
```

---

## WebDAV 方法使用

### 读取目录

使用：

```text
PROPFIND
```

### 读取文件

使用：

```text
GET
```

### 写入文件

使用：

```text
PUT
```

### 创建目录

使用：

```text
MKCOL
```

### 禁止使用

默认不使用：

```text
DELETE
MOVE
COPY
PROPPATCH
LOCK
UNLOCK
```

---

## Hermes 使用方式

Hermes 可以通过 shell tool 调用：

```bash
obsidian-cli new \
  --title "Hermes Telegram 流式卡顿排查" \
  --tag hermes \
  --tag telegram \
  --tag debug \
  --source telegram \
  --body "Hermes 流式输出在某个字卡住，不是整体延迟。"
```

也可以通过 stdin：

```bash
cat <<'EOF' | obsidian-cli new --title "Hermes Gateway Debug" --template troubleshooting
systemd 日志显示 gateway 停止时仍有其他 hermes 进程存在。
需要检查 tool_process 和 gateway 队列。
EOF
```

---

## Hermes Prompt 建议

可以在 Hermes 的系统提示词或 skill 中写入：

```text
当用户说“记到 Obsidian”、“保存到知识库”、“写入个人知识库”时：

1. 使用 obsidian-cli new 创建笔记
2. 默认写入 Inbox/Hermes/
3. 不要尝试写入 Notes/、Projects/、Index/ 等正式目录
4. 不要删除、移动、覆盖任何笔记
5. 标题应简洁明确
6. 正文使用 Markdown
7. 技术排查类内容使用 troubleshooting 模板
8. 标签不少于 2 个，不超过 6 个
9. 如果内容来自 Telegram，source 设置为 telegram
10. 如果内容来自 shell/log，source 设置为 shell 或 log
```

---

## 输出格式

普通文本输出：

```text
Created: Inbox/Hermes/2026-04-24-153012-hermes-gateway-debug.md
```

JSON 输出：

```bash
obsidian-cli new --title "测试" --body "hello" --json
```

```json
{
  "ok": true,
  "action": "created",
  "path": "Inbox/Hermes/2026-04-24-153012-test.md",
  "url": "https://example.com/webdav/Inbox/Hermes/2026-04-24-153012-test.md"
}
```

错误输出：

```json
{
  "ok": false,
  "error": "write_not_allowed",
  "message": "Writing to Notes/ is not allowed. Allowed write directories: Inbox/Hermes"
}
```

---

## 推荐子命令总览

```text
obsidian-cli init
obsidian-cli doctor
obsidian-cli ls [path]
obsidian-cli cat <path>
obsidian-cli search <query>
obsidian-cli new --title <title> [--body <body>]
obsidian-cli new --title <title> < stdin
obsidian-cli mkdir Inbox/Hermes/subdir
```

暂不实现：

```text
obsidian-cli delete
obsidian-cli move
obsidian-cli sync
obsidian-cli overwrite
```

---

## 示例工作流

### 记录 Telegram 内容

```bash
obsidian-cli new \
  --title "Hermes 与 Obsidian WebDAV 权限设计" \
  --source telegram \
  --tag hermes \
  --tag obsidian \
  --tag webdav \
  --body "Hermes 需要读取整个 vault，但只能向 Inbox/Hermes 写入。"
```

---

### 记录排障日志

```bash
journalctl -u hermes-gateway -n 100 --no-pager \
  | obsidian-cli new \
      --title "Hermes Gateway systemd 日志" \
      --template troubleshooting \
      --source log \
      --tag hermes \
      --tag systemd \
      --tag debug
```

---

### 搜索已有知识

```bash
obsidian-cli search "send_typing"
obsidian-cli search "Telegram gateway" --dir Notes
obsidian-cli search "sshd dead" --dir Troubleshooting
```

---

## 技术实现建议

### 推荐语言

优先推荐 Go。

原因：

```text
方便编译成单文件二进制
适合 Linux 服务器
方便给 Hermes 调用
没有 Python 虚拟环境依赖
HTTP/WebDAV 实现简单
适合你现有 Go 技术栈
```

可选语言：

```text
Go
Python
Rust
Node.js
```

最推荐：

```text
Go
```

---

## Go 项目结构建议

```text
obsidian-cli/
├── cmd/
│   └── obsidian-cli/
│       └── main.go
├── internal/
│   ├── config/
│   ├── webdav/
│   ├── markdown/
│   ├── pathguard/
│   ├── command/
│   └── search/
├── templates/
│   ├── basic.md
│   ├── troubleshooting.md
│   └── project.md
├── go.mod
├── README.md
└── config.example.yaml
```

---

## 核心模块

### config

负责读取：

```text
~/.config/obsidian-cli/config.yaml
环境变量
命令行参数
```

---

### webdav

负责：

```text
PROPFIND
GET
PUT
MKCOL
认证
超时
错误处理
```

---

### pathguard

负责：

```text
路径规范化
写入目录校验
禁止路径穿越
禁止写入 .obsidian
禁止绝对路径
```

---

### markdown

负责：

```text
生成 YAML frontmatter
生成标题
合并 tags
套用模板
slugify 文件名
```

---

### search

初期可以简单实现：

```text
递归 PROPFIND 获取 .md 文件
逐个 GET 内容
本地字符串匹配
返回 path + line number + snippet
```

后续可以优化：

```text
本地缓存索引
SQLite FTS
向量索引
RAG
```

---

## 第一阶段 MVP

第一阶段只实现：

```text
init
doctor
ls
cat
new
search
```

其中最重要的是：

```text
new
cat
search
doctor
```

MVP 验收标准：

```text
可以连接 WebDAV
可以读取 Vault 根目录
可以读取 Notes/Hermes.md
可以搜索全库 Markdown
可以向 Inbox/Hermes 写入新笔记
写入 Notes/test.md 会被 CLI 拒绝
写入 ../test.md 会被 CLI 拒绝
已存在文件不会被覆盖
```

---

## 第二阶段增强

```text
模板系统
JSON 输出
本地缓存
SQLite FTS 搜索
frontmatter 解析
标签搜索
双链提取
反向链接分析
每日总结辅助
```

---

## 第三阶段 Agent 化

```text
obsidian-cli context "Hermes gateway 卡顿"
obsidian-cli related "Telegram Bot"
obsidian-cli summarize --dir Inbox/Hermes
obsidian-cli backlinks Notes/Hermes.md
obsidian-cli graph --tag hermes
```

这些功能可以给 Hermes 提供更强的知识库上下文。

---

## 安全原则

`obsidian-cli` 必须遵守以下原则：

```text
默认只读
默认不覆盖
默认不删除
默认不移动
默认只写 Inbox/Hermes
所有写操作必须经过 pathguard
所有 destructive 操作默认不实现
WebDAV 密码不写入仓库
日志中不打印密码
```

---

## 最终定位

`obsidian-cli` 是 Hermes 和 Obsidian Vault 之间的安全桥梁。

它应该让 Hermes 成为：

```text
全库读者
单目录投稿者
结构化笔记生成器
知识库检索助手
```

而不是：

```text
Vault 管理员
自动重构器
自动删除器
自动同步器
```

最终效果：

```text
Hermes 可以理解你的整个 Obsidian 知识库
Hermes 可以把新内容写入 Inbox/Hermes
你可以在 Obsidian 中人工整理正式笔记
整个 Vault 不会被 Agent 随意修改
```
