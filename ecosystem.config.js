module.exports = {
  apps: [
    {
      name: 'cc-switch-tui',
      script: './target/release/cc-switch-tui',
      cwd: '.',
      exec_mode: 'fork',
      instances: 1,
      autorestart: true,
      env: {
        CC_SWITCH_NO_OPEN: '1',
        RUST_LOG: 'INFO',
      },
      max_restarts: 10,
      min_uptime: '5s',
    },
  ],
};
