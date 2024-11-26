const Configuration = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'scope-enum': [2, 'always', [
      'readme',
      'ci',
      'repo',
    ]],
  },
};

export default Configuration;
