const Configuration = {
  extends: ["@commitlint/config-conventional"],
  formatter: "@commitlint/format",

  rules: {
    "scope-enum": [2, "always", ["rust", "release", "docs", "ci", "xray", "podman", "docker"]],
    'scope-empty': [2, 'never'],
    'scope-case': [2, 'always', 'lower-case'],
    'subject-empty': [2, 'never'],
    'subject-min-length': [2, 'always', 10],
    "type-enum": [2, "always", ["feat", "chore", "fix", "perf", "refactor"]],
    'type-empty': [2, 'never'],
    'type-case': [2, 'always', 'lower-case'],
  },

  defaultIgnores: false,
};

export default Configuration;
