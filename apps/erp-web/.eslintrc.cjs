module.exports = {
  root: true,
  env: {
    browser: true,
    es2022: true
  },
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    ecmaFeatures: {
      jsx: true
    }
  },
  plugins: ['react', 'react-hooks', '@typescript-eslint', 'import', 'react-refresh'],
  extends: [
    'eslint:recommended',
    'plugin:react/recommended',
    'plugin:react-hooks/recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:import/recommended',
    'plugin:import/typescript',
    'prettier'
  ],
  settings: {
    react: {
      version: 'detect'
    },
    'import/resolver': {
      typescript: {
        project: './tsconfig.json'
      }
    }
  },
  rules: {
    '@typescript-eslint/consistent-type-imports': 'off',
    '@typescript-eslint/no-explicit-any': 'error',
    'import/no-duplicates': 'off',
    'import/order': 'off',
    'react-hooks/exhaustive-deps': 'off',
    'react/react-in-jsx-scope': 'off',
    'react-refresh/only-export-components': 'off'
  }
};
