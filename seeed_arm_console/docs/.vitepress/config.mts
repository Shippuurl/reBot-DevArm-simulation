import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'reBot-DevArm Simulation',
  description: '基于 Pinocchio、ProxSuite、MuJoCo 与 Rerun、通过 gRPC SDK 接入的仿真工作站文档',
  cleanUrls: true,
  lastUpdated: true,
  themeConfig: {
    nav: [
      { text: '指南', link: '/guide/introduction' },
      { text: '实施计划', link: '/simulation-work-plan' },
      { text: 'Rerun 数据', link: '/panels/rerun-viewer' },
      { text: '后端与仿真', link: '/backend/simulation' },
      { text: 'SDK', link: '/sdk/python' },
      { text: '架构', link: '/architecture/c4-model' },
      { text: '源码构建', link: '/guide/simulation' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: '入门指南',
          items: [
            { text: '项目介绍', link: '/guide/introduction' },
            { text: '仿真工作站', link: '/guide/simulation' },
          ],
        },
      ],
      '/panels/': [
        {
          text: '上位机模块',
          items: [
            { text: 'Rerun 视图', link: '/panels/rerun-viewer' },
          ],
        },
      ],
      '/backend/': [
        {
          text: '后端与算法',
          items: [
            { text: '规划与仿真', link: '/backend/simulation' },
          ],
        },
      ],
      '/sdk/': [
        {
          text: '官方 SDK',
          items: [
            { text: 'Python SDK', link: '/sdk/python' },
            { text: 'C++ SDK', link: '/sdk/cpp' },
            { text: 'Rust SDK', link: '/sdk/rust' },
          ],
        },
      ],
      '/architecture/': [
        {
          text: '架构视图',
          items: [
            { text: 'C4 模型', link: '/architecture/c4-model' },
          ],
        },
      ],
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/your-org/seeed-arm-console' },
    ],
    footer: {
      message: 'reBot-DevArm 仿真工作站文档',
      copyright: 'Copyright © 2026 reBot-DevArm contributors',
    },
    search: { provider: 'local' },
  },
})
