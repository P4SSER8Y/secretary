# secretary

![GitHub Actions](https://github.com/P4SSER8Y/secretary/actions/workflows/develop.yml/badge.svg)
![GitHub Actions](https://github.com/P4SSER8Y/secretary/actions/workflows/release.yml/badge.svg)

a server runned at home

## kindle

Generate kindle screen saver as dashboard. Inspired and powered by [pascalw/kindle-dash](https://github.com/pascalw/kindle-dash)

设备每次唤醒会先拉配置再刷新：

- `GET /kindle/cfg` 返回 JSON，`?format=text` 返回 `key=value`（Kindle 端用）
- `POST /kindle/cfg` 表单更新，支持 `sleep`（1/0 自动休眠）、`interval`（下次唤醒秒数）、`style`（样式，空=默认）

```sh
curl -X POST -d "interval=3600" https://s.32323235.xyz/kindle/cfg
```

## bark

Send some message to iPhone via [bark](https://github.com/finb/bark).

## qweather

Fetch weather forecast, powered by [QWeather](https://dev.qweather.com/).

## let_server_run
Communicate via Wechat, powered by [LetServerRun](https://letserver.run/).

## inbox
A pastebin supported files and text.

## tsdb
A wrapper for InfluxDB2.

## meme
Meme Heap
