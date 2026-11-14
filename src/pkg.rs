pub type PkgID = u64;

pub const DEFAULT_KEY: &str = "_";

/// 生成跨进程/跨机器无冲突的 pkg id（同时也是 `wp_event_id` 的来源）。
///
/// 直接复用 [`wp_model_core::event_id::next_wp_event_id`] 的布局
/// `秒(32) | 实例号(10) | 每秒计数器(22)`：跨实例靠实例号（`WP_INS_ID`）隔离，
/// 同实例内靠每秒计数器递增。历史实现是「秒级时间前缀 + 进程内计数器」，
/// 两台机器在同一秒启动时会生成完全相同的序列，导致 `wp_event_id` 重复
/// （如 MySQL 主键冲突）。
///
/// 不再裁掉最高位：布局保证 ID 在 2038 年以前都落在 `i64` 正区间内，裁位会在
/// 越界后静默改变取值、反而制造重复。
pub fn gen_pkg_id() -> u64 {
    wp_model_core::event_id::next_wp_event_id()
}

#[cfg(test)]
mod tests {
    use crate::pkg::gen_pkg_id;

    #[test]
    fn test_gen_id() {
        let a = gen_pkg_id();
        let b = gen_pkg_id();
        assert!(
            b > a,
            "pkg id should be monotonically increasing: {a} -> {b}"
        );
    }

    #[test]
    fn pkg_id_uses_event_layout() {
        let id = gen_pkg_id();
        let (secs, lane, _) = wp_model_core::event_id::decompose_event_id(id);
        assert!(secs > 0, "secs 段应取自时间戳");
        assert!(lane <= wp_model_core::event_id::MAX_LANE);
        assert_eq!(
            wp_model_core::event_id::INS_ID_BASE + lane,
            wp_model_core::event_id::instance_id(),
            "pkg id 必须与 wp_event_id 使用同一实例号"
        );
    }
}
