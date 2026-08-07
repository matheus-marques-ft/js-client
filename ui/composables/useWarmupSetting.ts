export async function useWarmupSetting() {
  try {
    // Precompile the layouts and pages related to /setting, to reduce the blank screen on first open
    await Promise.all([
      import("@/layouts/setting.vue"),
      import("@/pages/setting/index.vue"),
      import("@/pages/setting/application/ssh.vue"),
      import("@/pages/setting/application/telnet.vue"),
      import("@/pages/setting/application/sftp.vue"),
      import("@/pages/setting/application/rdp.vue"),
      import("@/pages/setting/application/vnc.vue"),
      import("@/pages/setting/application/mysql.vue"),
      import("@/pages/setting/application/mongodb.vue"),
      import("@/pages/setting/application/redis.vue"),
      import("@/pages/setting/application/pg.vue"),
      import("@/pages/setting/application/oracle.vue"),
      import("@/pages/setting/application/sqlserver.vue")
    ]);
  } catch {}
}
