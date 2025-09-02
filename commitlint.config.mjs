const Configuration = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'scope-enum': [2, 'always', [
      'readme',
      'ci',
      'repo',
      'commitlint',
      'query-root',
      'mutation-root',
      'region',
      'crag',
      'sector',
      'formation',
      'climb',
    ]],
  },
};

export default Configuration;
