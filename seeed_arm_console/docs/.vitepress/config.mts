import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'Seeed Arm Console',
  description: '基于 Rust、egui、ROS 2 Jazzy 与 Rerun 的机器人上位机文档',
  cleanUrls: true,
  lastUpdated: true,
  themeConfig: {
    nav: [
      { text: '指南', link: '/guide/introduction' },
      { text: '实施计划', link: '/plan' },
      { text: '上位机面板', link: '/panels/control' },
      { text: '后端与仿真', link: '/backend/grpc-api' },
      { text: '架构', link: '/architecture/c4-model' },
      { text: '源码构建', link: '/dev/build-source' },
      { text: 'Rust API', link: '/dev/rust-api' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: '入门指南',
          items: [
            { text: '项目介绍', link: '/guide/introduction' },
            { text: '快速开始', link: '/guide/quick-start' },
            { text: '系统架构概览', link: '/guide/architecture' },
          ],
        },
      ],
      '/panels/': [
        {
          text: '上位机模块',
          items: [
            { text: '控制面板', link: '/panels/control' },
            { text: 'Jog 与轨迹', link: '/panels/jog' },
            { text: 'Rerun 视图', link: '/panels/rerun-viewer' },
          ],
        },
      ],
      '/backend/': [
        {
          text: '后端与算法',
          items: [
            { text: 'gRPC API', link: '/backend/grpc-api' },
            { text: '状态机与安全', link: '/backend/state-machine' },
            { text: 'OpenRAVE 与 MuJoCo', link: '/backend/simulation' },
          ],
        },
      ],
      '/architecture/': [
        {
          text: '架构视图',
          items: [
            { text: 'C4 模型', link: '/architecture/c4-model' },
            { text: '4+1 视图', link: '/architecture/4plus1' },
          ],
        },
      ],
      '/dev/': [
        {
          text: '开发参考',
          items: [
            { text: '源码构建', link: '/dev/build-source' },
            { text: 'Rust API 文档', link: '/dev/rust-api' },
          ],
        },
      ],
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/your-org/seeed-arm-console' },
    ],
    footer: {
      message: 'Seeed Arm Console 文档',
      copyright: 'Copyright © 2026 Seeed Arm Console contributors',
    },
    search: { provider: 'local' },
  },
})
