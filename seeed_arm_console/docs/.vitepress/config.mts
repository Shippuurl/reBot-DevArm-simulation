import { defineConfig } from 'vitepress'

const overviewItems = [
  { text: '项目简介', link: '/guide/introduction' },
  { text: '系统架构', link: '/architecture/c4-model' },
  { text: '设计决策（ADR）', link: '/architecture/decisions' },
]

const quickStartItems = [
  { text: '仿真工作站', link: '/guide/simulation' },
  { text: 'Python SDK', link: '/sdk/python' },
  { text: 'C++ SDK', link: '/sdk/cpp' },
  { text: 'Rust SDK', link: '/sdk/rust' },
]

const moduleItems = [
  { text: '规划与仿真', link: '/backend/simulation' },
  { text: 'Rerun Viewer', link: '/panels/rerun-viewer' },
  { text: 'SDK 与协议边界', link: '/architecture/sdk-boundary' },
]

const maintenanceItems = [
  { text: '源码构建', link: '/development/build' },
]

export default defineConfig({
  lang: 'zh-CN',
  title: 'reBot-DevArm Simulation',
  description: 'B601-RS 机械臂仿真、规划与可视化平台的 SDK 接入文档',
  cleanUrls: true,
  lastUpdated: true,
  themeConfig: {
    nav: [
      { text: '项目概览', items: overviewItems },
      { text: '快速开始', items: quickStartItems },
      { text: '核心模块', items: moduleItems },
      { text: '开发与维护', items: maintenanceItems },
    ],
    sidebar: [
      {
        text: '项目概览',
        items: overviewItems,
      },
      {
        text: '快速开始',
        items: quickStartItems,
      },
      {
        text: '核心模块',
        items: moduleItems,
      },
      {
        text: '开发与维护',
        items: maintenanceItems,
      },
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Shippuurl/reBot-DevArm-simulation' },
    ],
    footer: {
      message: 'reBot-DevArm 仿真工作站文档',
      copyright: 'Copyright © 2026 reBot-DevArm contributors',
    },
    search: { provider: 'local' },
  },
})
