mod etag;
mod sync;

use std::collections::HashSet;
use std::path::PathBuf;

pub use sync::SyncResult;

/// 本地同步存储 — 维护 S3 对象的本地副本 + sled ETag 缓存
///
/// S3 是 source of truth，本地文件是副本。ETag 缓存在 sled 中，
/// 用于增量同步时跳过未变更的对象。
pub struct SyncStore {
    db_prefix: String,
    local_base: PathBuf,
}

impl SyncStore {
    /// 创建新的 SyncStore。
    ///
    /// * `db_prefix` — sled 中 ETag key 的前缀（调用方保证唯一，如 `"meme/s3_etag/"`）
    /// * `local_base` — 本地文件根目录（S3 key 路径直接拼接到此目录下）
    pub fn new(db_prefix: &str, local_base: PathBuf) -> Self {
        Self {
            db_prefix: db_prefix.to_string(),
            local_base,
        }
    }

    /// s3_key → 本地文件路径 (local_base / s3_key)
    pub fn local_path(&self, s3_key: &str) -> PathBuf {
        self.local_base.join(s3_key)
    }

    // ── 核心同步 ──

    /// 增量同步：列出 S3 对象 → ETag 比对 → 仅下载变更。
    ///
    /// **不执行清理** — 返回的 `SyncResult.known_keys` 可供调用方聚合后调用 `cleanup()`。
    ///
    /// * `bucket` — 已配置好的 S3 Bucket（调用方负责设置 path_style、listobjects_v1 等）
    /// * `prefix` — S3 前缀过滤（如 `"meta/alice/_plain/"`）
    /// * `delimiter` — S3 分隔符，`None` 表示递归列出
    pub async fn sync(
        &self,
        bucket: &s3::Bucket,
        prefix: &str,
        delimiter: Option<&str>,
    ) -> anyhow::Result<SyncResult> {
        sync::sync_from_s3(self, bucket, prefix, delimiter).await
    }

    // ── 写操作 ──

    /// 上传到 S3 + 写本地副本 + 缓存 ETag。
    ///
    /// ETag 从 S3 put_object 返回值获取；若未返回则跳过 ETag 缓存（下次 sync 会重下载）。
    pub async fn push(
        &self,
        bucket: &s3::Bucket,
        key: &str,
        data: &[u8],
    ) -> anyhow::Result<()> {
        sync::push_to_s3(self, bucket, key, data).await
    }

    /// 删除本地副本 + 清除 ETag（S3 上的删除由调用方处理）。
    pub fn invalidate(&self, key: &str) {
        let local_path = self.local_path(key);
        if local_path.exists() {
            if let Err(e) = std::fs::remove_file(&local_path) {
                log::warn!("s3_sync: failed to remove local file {:?}: {}", local_path, e);
            }
        }
        etag::remove_cached_etag(&self.db_prefix, key);
    }

    // ── 清理 ──

    /// 合并清理：删除本地过期文件 + sled 过期 ETag。
    /// 应在所有 `sync()` 调用完成后调用一次。
    pub fn cleanup(&self, all_known_keys: &HashSet<String>) {
        self.clean_stale_files(all_known_keys);
        self.purge_stale_etags(all_known_keys);
    }

    /// 删除 sled 中所有不在 `known_keys` 里的 ETag（key 为完整 S3 key）。
    pub fn purge_stale_etags(&self, known_keys: &HashSet<String>) {
        etag::purge_stale_etags(&self.db_prefix, known_keys);
    }

    /// 递归删除 `local_base` 下不在 `known_keys` 里的文件，并清理空目录。
    /// `known_keys` 中的 key 为完整 S3 key。
    pub fn clean_stale_files(&self, known_keys: &HashSet<String>) {
        sync::clean_stale_files(&self.local_base, &self.local_base, known_keys);
    }
}
