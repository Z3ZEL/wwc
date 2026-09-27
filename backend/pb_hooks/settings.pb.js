/// <reference path="../pb_data/types.d.ts" />

// Instance settings, applied on every boot (ADR 0012). PocketBase keeps them in its settings
// table; this makes the repo and the deployment env the source of truth. In order:
//  1. `pb_settings.json`: committed, non-secret settings (rate limits, logs, batch API…), in
//     PocketBase's own settings shape.
//  2. `PB_SETTINGS_FILE`, if set: a deployment's own overrides (a mounted JSON file, same shape).
//  3. `pb_settings.dev.json` when `PB_DEV=1`.
//  4. env vars (S3, backups, SMTP, app URL; full list in .env.example): secrets and
//     per-environment values. A section is only touched when its env vars are set.
// Anything not covered stays as set in the dashboard. Upload limits are field options in the
// schema, not settings: change them with a migration.

onBootstrap((e) => {
  e.next();

  // Handlers run in their own scope: helpers must be defined inside.
  const env = (name) => $os.getenv(name).trim();
  const isSet = (name) => env(name) !== "";
  const bool = (name) => ["1", "true", "yes"].includes(env(name).toLowerCase());
  const int = (name, fallback) => (isSet(name) ? parseInt(env(name), 10) : fallback);

  // `prefix` = "PB_S3" or "PB_BACKUPS_S3". Returns null when the section isn't configured.
  const s3FromEnv = (prefix) => {
    if (!isSet(`${prefix}_ENABLED`)) return null;
    return {
      enabled: bool(`${prefix}_ENABLED`),
      bucket: env(`${prefix}_BUCKET`),
      region: env(`${prefix}_REGION`),
      endpoint: env(`${prefix}_ENDPOINT`),
      accessKey: env(`${prefix}_ACCESS_KEY`),
      secret: env(`${prefix}_SECRET`),
      forcePathStyle: bool(`${prefix}_FORCE_PATH_STYLE`),
    };
  };
  const applyS3 = (target, cfg) => {
    for (const key of Object.keys(cfg)) target[key] = cfg[key];
  };

  const settings = e.app.settings();
  const changed = [];

  // Deep merge onto the settings object. Arrays and scalars replace; an unknown key (typo)
  // stops the boot rather than being silently ignored.
  const merge = (target, patch, path) => {
    for (const key of Object.keys(patch)) {
      const value = patch[key];
      const where = path ? `${path}.${key}` : key;
      if (target[key] === undefined) throw new Error(`unknown PocketBase setting "${where}"`);
      if (value !== null && typeof value === "object" && !Array.isArray(value)) merge(target[key], value, where);
      else target[key] = value;
    }
  };
  const mergeFile = (file, required) => {
    let raw;
    try {
      raw = toString($os.readFile(file));
    } catch (err) {
      if (required) throw new Error(`can't read settings file ${file}: ${err}`);
      return;
    }
    try {
      merge(settings, JSON.parse(raw), "");
    } catch (err) {
      throw new Error(`${file}: ${err}`);
    }
    changed.push(file);
  };
  mergeFile("/pb_settings.json", true);
  if (isSet("PB_SETTINGS_FILE")) mergeFile(env("PB_SETTINGS_FILE"), true);
  if (env("PB_DEV") === "1") mergeFile("/pb_settings.dev.json", true);

  // Public PocketBase URL: the links in emails point to its confirmation pages.
  const meta = { appURL: "PB_APP_URL", appName: "PB_APP_NAME", senderName: "PB_SENDER_NAME", senderAddress: "PB_SENDER_ADDRESS" };
  for (const key of Object.keys(meta)) {
    if (isSet(meta[key])) {
      settings.meta[key] = env(meta[key]);
      changed.push(key);
    }
  }

  // Uploaded files (campsite photos).
  const files = s3FromEnv("PB_S3");
  if (files) {
    applyS3(settings.s3, files);
    changed.push(`s3 ${files.enabled ? "on" : "off"}`);
  }

  // Backups: their own bucket if PB_BACKUPS_S3_* is set, else the files bucket.
  const backups = s3FromEnv("PB_BACKUPS_S3") || files;
  if (backups) {
    applyS3(settings.backups.s3, backups);
    changed.push(`backups s3 ${backups.enabled ? "on" : "off"}`);
  }
  if (isSet("PB_BACKUPS_CRON") || (backups && backups.enabled)) {
    settings.backups.cron = isSet("PB_BACKUPS_CRON") ? env("PB_BACKUPS_CRON") : "0 3 * * *";
    settings.backups.cronMaxKeep = int("PB_BACKUPS_MAX_KEEP", 7);
    changed.push(`backups cron "${settings.backups.cron}"`);
  }

  // Outgoing mail (email confirmation, password reset).
  if (isSet("PB_SMTP_ENABLED")) {
    settings.smtp.enabled = bool("PB_SMTP_ENABLED");
    settings.smtp.host = env("PB_SMTP_HOST");
    settings.smtp.port = int("PB_SMTP_PORT", 587);
    settings.smtp.username = env("PB_SMTP_USERNAME");
    settings.smtp.password = env("PB_SMTP_PASSWORD");
    settings.smtp.tls = bool("PB_SMTP_TLS");
    settings.smtp.authMethod = env("PB_SMTP_AUTH_METHOD") || "PLAIN";
    changed.push(`smtp ${settings.smtp.enabled ? "on" : "off"}`);
  }

  if (isSet("PB_TRUSTED_PROXY_HEADERS")) {
    settings.trustedProxy.headers = env("PB_TRUSTED_PROXY_HEADERS").split(",").map((h) => h.trim()).filter(Boolean);
    changed.push("trusted proxy");
  }

  if (!changed.length) return;
  e.app.save(settings);
  e.app.logger().info("settings from env", "applied", changed.join(", "));
});
