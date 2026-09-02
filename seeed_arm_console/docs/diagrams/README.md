# 图表

PlantUML 源文件描述规划数据流、SDK 边界和 Rerun 实体树；生成的 SVG 放在
`docs/public/diagrams/`，由 VitePress 页面引用。

## 生成 SVG

```bash
cd docs/diagrams
java -Djava.awt.headless=true \
  -jar /path/to/plantuml.jar \
  -tsvg -o ../public/diagrams *.puml
```

修改图表时同时提交对应的 `.puml` 和 `.svg` 文件。系统边界图见[系统架构](/architecture/c4-model)，
实体树见[Rerun Viewer](/panels/rerun-viewer)。
