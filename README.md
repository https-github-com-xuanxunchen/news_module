# 软件工程课设新闻内容模块

## 部署

### 前置

- Docker Compose
- Docker


### 启动

```sh
$ cd docker && docker-compose up --build
```

### 新闻爬取

```sh
$ cd docker && docker-compose exec news_service /crawler
```
