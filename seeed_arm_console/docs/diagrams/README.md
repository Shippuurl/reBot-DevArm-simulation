# PlantUML 图表

架构和数据流图的源文件位于此目录，生成的 SVG 位于 `docs/public/diagrams/`。

使用仓库统一的 PlantUML：

```bash
java -Djava.awt.headless=true \
  -jar /media/shippuu/Date/plantUML/plantuml.jar \
  -tsvg -o ../public/diagrams *.puml
```

提交图表修改时同时更新 `.puml` 和生成的 `.svg`。
