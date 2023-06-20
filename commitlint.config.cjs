const Configuration = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "scope-enum": [
      2,
      "always",
      [
        "demos",
        "root",
        "getting-started-rust-cloudflare-workers",
        "mux-serverless-webhook-updates",
        "rust-ci-tooling",
        "rust-cloudflare-worker-logging",
        "rust-cloudflare-workers-turnstile-example"
      ],
    ],
    "scope-empty": [2, "never"],
  },
};

module.exports = Configuration;
