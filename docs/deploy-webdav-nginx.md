# Obsidian Vault WebDAV/Nginx 端部署

本文档面向存放 Obsidian Vault 的服务端。服务端不需要部署 `webdav-cli` 常驻进程；需要部署或保留一个已有 WebDAV 服务，让 Hermes 端的 `webdav-cli` 通过 HTTP/WebDAV 访问 Vault。

目标权限模型：

```text
GET / HEAD / OPTIONS / PROPFIND 允许读取整个 Vault
GET / HEAD / OPTIONS / PROPFIND / PUT / MKCOL / DELETE / MOVE / COPY / PROPPATCH / LOCK / UNLOCK 允许 /Inbox/Hermes/
DELETE / MOVE / COPY / PROPPATCH / LOCK / UNLOCK 在 /Inbox/Hermes/ 之外禁止
```

如果 Obsidian 客户端已经直接使用 `/webdav/`，不要把 `/webdav/` 改成上述受限权限，否则会影响 Obsidian 自身的同步、重命名、删除、附件写入等正常功能。推荐做法是：

```text
/webdav/           保持原样，继续给 Obsidian 客户端使用
/obsidian-webdav/  新增受限入口，只给 webdav-cli / Hermes 使用
```

## 1. Vault 目录结构

推荐结构：

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

确保 Nginx worker 用户能读取 Vault，并能写入 `Inbox/Hermes`：

```bash
sudo mkdir -p /srv/obsidian/ObsidianVault/Inbox/Hermes
sudo chown -R nginx:nginx /srv/obsidian/ObsidianVault/Inbox/Hermes
sudo chmod -R u+rwX /srv/obsidian/ObsidianVault/Inbox/Hermes
```

发行版的 Nginx 用户可能是 `www-data`，请按实际环境替换。

## 2. Nginx WebDAV 模块要求

普通静态文件服务不支持完整 WebDAV。Nginx 内置 `ngx_http_dav_module` 主要覆盖 `PUT`、`DELETE`、`MKCOL`、`COPY`、`MOVE`，`PROPFIND` 通常需要额外的 WebDAV 扩展模块，例如 `nginx-dav-ext-module`。

如果你已经有 Nextcloud、Apache WebDAV、rclone serve webdav、Alist 等现成 WebDAV 服务，可以不使用 Nginx；但仍必须满足全库可读、`Inbox/Hermes` 拥有完整 HTTP/WebDAV 方法权限、正式目录只读的权限模型。

部署前确认当前 Nginx 支持 WebDAV 读目录：

```bash
nginx -V 2>&1 | grep -E 'dav|nginx-dav-ext-module'
```

如果没有 `PROPFIND` 支持，`webdav-cli ls/search/doctor` 将无法正常工作。

## 3. 已有 WebDAV 服务的 Nginx 配置

如果你的 WebDAV 已经由 `127.0.0.1:8001`、Alist、rclone serve webdav、Apache WebDAV、Nextcloud 等服务提供，可以在当前 HTTPS `server` 中新增以下两个 `location`。这不会改动原有 `/webdav/` 入口。

```nginx
location ^~ /obsidian-webdav/Inbox/Hermes/ {
    limit_except GET HEAD OPTIONS PROPFIND PUT MKCOL DELETE MOVE COPY PROPPATCH LOCK UNLOCK {
        deny all;
    }

    proxy_pass http://127.0.0.1:8001/Inbox/Hermes/;
    proxy_http_version 1.1;

    proxy_set_header Host              $proxy_host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header Authorization     $http_authorization;

    proxy_hide_header Allow;
    add_header Allow "GET, HEAD, OPTIONS, PROPFIND, PUT, MKCOL, DELETE, MOVE, COPY, PROPPATCH, LOCK, UNLOCK" always;

    proxy_buffering off;
    proxy_request_buffering off;
}

location ^~ /obsidian-webdav/ {
    limit_except GET HEAD OPTIONS PROPFIND {
        deny all;
    }

    proxy_pass http://127.0.0.1:8001/;
    proxy_http_version 1.1;

    proxy_set_header Host              $proxy_host;
    proxy_set_header X-Real-IP         $remote_addr;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header Authorization     $http_authorization;

    proxy_hide_header Allow;
    add_header Allow "GET, HEAD, OPTIONS, PROPFIND" always;

    proxy_buffering off;
    proxy_request_buffering off;
}
```

注意：

- `/obsidian-webdav/Inbox/Hermes/` 要开放完整 HTTP/WebDAV 方法：`GET`、`HEAD`、`OPTIONS`、`PROPFIND`、`PUT`、`MKCOL`、`DELETE`、`MOVE`、`COPY`、`PROPPATCH`、`LOCK`、`UNLOCK`。
- 不要在 `/obsidian-webdav/Inbox/Hermes/` 之外开放 `PUT`、`MKCOL`、`DELETE`、`MOVE`、`COPY`、`PROPPATCH`、`LOCK`、`UNLOCK`。
- `/obsidian-webdav/Inbox/Hermes/` 的 location 必须比 `/obsidian-webdav/` 更具体，并放在它前面。
- `proxy_pass` 后面的 `127.0.0.1:8001` 要替换成你当前 WebDAV 服务的真实监听地址。
- 如果后端 WebDAV 已经有 Basic Auth，`Authorization` 会透传给后端；如果认证在 Nginx 层完成，可以按你的现有配置保留 `auth_basic`。
- `webdav-cli` 自身也会做路径校验，但服务端权限仍然必须正确配置。

## 4. CLI 配置

Hermes 端的配置应指向受限入口：

```yaml
webdav:
  base_url: "https://example.com/obsidian-webdav/"
  username: "hermes"
  password_env: "OBSIDIAN_WEBDAV_PASSWORD"
```

如果你的域名是 `www.tencent.lemon9527.top`，则写成：

```yaml
webdav:
  base_url: "https://www.tencent.lemon9527.top/obsidian-webdav/"
```

## 5. 从 Hermes 端验证

在 Hermes 机器设置：

```bash
export OBSIDIAN_WEBDAV_PASSWORD='your-webdav-password'
webdav-cli doctor
webdav-cli ls
webdav-cli new --title "WebDAV 验证" --body "hello"
```

`doctor` 会发起 `OPTIONS`、`PROPFIND` 和 `PUT` 探测，并校验 `Inbox/Hermes` 的 `Allow` 方法集合包含完整 HTTP/WebDAV 权限。正常配置下应显示：

```text
[OK] Inbox/Hermes full HTTP permissions
```

安全验收：

```bash
webdav-cli new --title "越权测试" --dir Notes --body "should fail"
webdav-cli new --title "穿越测试" --dir ../Notes --body "should fail"
```

这两个命令都应该失败，且不应在 Vault 正式目录中创建文件。

最后在服务器上验证并重载 Nginx：

```bash
sudo nginx -t
sudo systemctl reload nginx
```

只要不修改原有 `/webdav/` 的 `location`，Obsidian 直接使用 WebDAV 的功能不会被这组 `/obsidian-webdav/` 受限入口影响。
