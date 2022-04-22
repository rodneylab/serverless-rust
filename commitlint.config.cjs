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
      ],
    ],
    "scope-empty": [2, "never"],
  },
};

module.exports = Configuration;
