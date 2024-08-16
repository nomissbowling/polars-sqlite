// dot -v -Kdot -Tpng res\dtypes.v -ores\dtypes.png
digraph dtypes {
  graph [
    charset = "UTF-8";
    label = "conversion between polars DataFrame DataTypes and sqlite::Value",
    labelloc = "t",
    labeljust = "c",
    bgcolor = "#336699",
    fontcolor = violet,
    fontsize = 24,
    style = "filled",
    rankdir = TB,
    margin = 0.2,
    splines = spline,
    ranksep = 1.0,
    nodesep = 0.9
  ];

  node [
    colorscheme = "rdylgn11"
    style = "solid,filled",
    fontsize = 20,
    fontcolor = 6,
    fontname = "Migu 1M",
    color = 7,
    fillcolor = 11,
    fixedsize = true,
    height = 1.2,
    width = 3.6
  ];

  edge [
    style = solid,
    fontsize = 16,
    fontcolor = cyan,
    fontname = "Migu 1M",
    color = white,
    labelfloat = true,
    labeldistance = 2.5,
    labelangle = 70
  ];

  // node define
  sqlite3Table [shape=box];
  Values [label="Vec<(&str, sqlite::Value)>", shape=box];
  sqliteRow [label="sqlite::Row", shape=box];
  Tester [shape=circle];
  RecTester [shape=doublecircle];
  StTester [shape=Msquare];
  Row [label="polars::frame::row::Row", shape=polygon, sides=6];

  // edge define
  sqlite3Table -> sqliteRow[label="read", arrowtail=crow, dir=back,
    style=dashed, color=lime, fontcolor=lime];
  Values -> sqlite3Table[label="write", arrowhead=normal,
    style=bold, color=navy];
  StTester -> Values[label=".to_sqlite3_vec()", arrowhead=normal,
    style=bold, color=navy];
  sqliteRow -> Tester[label="to_tester()", arrowhead=tee,
    style=dashed, color=lime, fontcolor=lime];
  sqliteRow -> RecTester[taillabel="from", arrowtail=crow, dir=back];
  sqliteRow -> StTester[taillabel="from", arrowtail=crow, dir=back];
  Tester -> RecTester[taillabel="from", arrowtail=crow, dir=back,
    style=dotted, color=orange, fontcolor=orange];
  Tester -> StTester[taillabel="from", arrowtail=crow, dir=back,
    style=dashed, color=lime, fontcolor=lime];
  StTester -> Tester[label=".to_tester()", arrowhead=normal,
    color=orange, fontcolor=orange];
  RecTester -> Row[taillabel=".into_iter().collect()", arrowhead=normal];
  StTester -> Row[taillabel=".into_vec()", arrowhead=normal,
    style=dashed, color=lime, fontcolor=lime];
  Row -> StTester[label="sttester_from()", arrowtail=crow, dir=back,
    style=bold, color=navy];

  {rank=min; sqlite3Table}
  {rank=same; Tester, sqliteRow, Values}
  {rank=same; RecTester, StTester}
  {rank=max; Row}
}
