# Windfoil ― 輪郭線からピクセルの濃さを求める

フォントや SVG の輪郭線は、画面のピクセルの境目にぴったり合うとは限りません。輪郭線がピクセルを半分だけ横切ることもあれば、細い線がピクセルの端をかすめることもあります。そのような場合でも、形をギザギザにせず、どれくらい塗ればよいかを求める必要があります。

Windfoil は、輪郭線をたくさんの三角形に分けて塗る代わりに、**輪郭線が作る「巻き数」をピクセルの中で積分し、ピクセルをどの程度塗るかを計算する**方法です。ここでは、数式をできるだけ使わずに、巻き数、塗りの規則、積分、そして実装の流れを順に見ていきます。

> この記事は、[Windfoil の公開仕様](https://github.com/texel-org/windfoil-algorithm/blob/main/docs/ALGORITHM.md)を参考にしつつ、炊紙（kashikishi）の実装を説明します。実装は元のコードのコピーではなく、二次ベジェ曲線と WebGPU のシェーダーを使って組み込んだものです。

## 炊紙では描画方式を切り替えられる

炊紙では、従来の三角形描画を使う **Legacy** と、この記事で説明する **Windfoil** をバックエンドとして選択できます。既定は Legacy で、Windfoil が従来の処理を全面的に置き換えるわけではありません。

操作メニューの「描画方式の変更」から、再起動せずに切り替えられます。アクション名は `system:change-rendering-backend-ui`、直接指定する場合は `system:change-rendering-backend` に `legacy` または `windfoil` を渡します。選択はエディター設定の `rendering_backend` に `Legacy` / `Windfoil` として保存され、リサイズ・画面品質・塗り規則の変更後も維持されます。設定項目がない既存の設定ファイルでは Legacy を使います。

ライブラリからは `RasterizerPipeline::new_with_backend` または `RasterizerRenderrer::new_with_backend` で選択します。既存の `new` は Legacy のままです。PNG 出力にも `render_vector_vertex_to_png_with_backend` とその非同期版があり、同じ輪郭を両方式で比較できます。ただし、アンチエイリアスの計算方法が異なるため、両方式で全く同じ画素値になることは保証しません。

## まず「輪郭線の中」を決めたい

輪郭線は、紙に描いた閉じた線のようなものです。線の内側を塗りたいのですが、線が交差したり、内側に穴があったりすると、「内側とはどちらか」を単純には決められません。

たとえば、調べたい点から外へ向かって、まっすぐな半直線を伸ばしてみます。その線を輪郭線が横切るたびに、輪郭線が半直線を上向きに横切ったか、下向きに横切ったかに応じて `+1` または `−1` を足します。この符号付きの合計が、その点の **巻き数（winding number）** です。巻き数は点の周りを輪郭線が何周するかを表しますが、線がその場で円を描くという意味ではありません。

次の図では、点 P から右向きに半直線を伸ばしています。線を横切る輪郭線の向きが、巻き数の符号を決めます。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 620 245" role="img" aria-labelledby="windfoil-ray-title windfoil-ray-desc">
  <title id="windfoil-ray-title">点から輪郭線へ伸ばす半直線と巻き数</title>
  <desc id="windfoil-ray-desc">閉じた輪郭線の内側の点Pから右向きに半直線を引く。輪郭線を横切るとき、上向きの交差はプラス、下向きの交差はマイナスとして数える。</desc>
  <defs>
    <marker id="ray-arrow" markerWidth="10" markerHeight="8" refX="8" refY="4" orient="auto">
      <path d="M0,0 L9,4 L0,8 Z" fill="#52606d"/>
    </marker>
  </defs>
  <rect width="620" height="245" rx="12" fill="#f8fafc"/>
  <path d="M190 48 C285 45 385 75 410 123 C436 174 344 205 238 185 C157 170 133 107 190 48 Z" fill="#dbeafe" stroke="#2563eb" stroke-width="4"/>
  <line x1="286" y1="132" x2="555" y2="132" stroke="#52606d" stroke-width="2.5" marker-end="url(#ray-arrow)"/>
  <circle cx="286" cy="132" r="6" fill="#111827"/>
  <text x="270" y="157" font-size="17" fill="#111827">点 P</text>
  <circle cx="400" cy="132" r="6" fill="#dc2626"/>
  <text x="363" y="111" font-size="16" fill="#991b1b">交差：向きを加算</text>
  <text x="28" y="225" font-size="15" fill="#334155">交差の向きによって +1 / −1 を足し、合計が巻き数になる</text>
</svg>

半直線は図では右向きですが、実際にはどの向きに伸ばしてもかまいません。交差の数え方を一貫させれば、点が輪郭線のどこを何回囲まれているかがわかります。輪郭線に囲まれていない点では、交差の寄与が打ち消し合い、巻き数は 0 になります。

### 巻き数は「内側か外側か」だけではない

単純な輪郭線の内側なら、巻き数は通常 `+1` または `−1` です。どちらの符号になるかは、輪郭線をたどる向きで変わります。輪郭線を反対向きにたどると、すべての符号が反転します。

輪郭線が同じ点を二重に囲むように重なると、巻き数が `+2` や `−2` になることもあります。また、逆向きに描かれた輪郭線が重なると、`+1` と `−1` が打ち消し合う場所もできます。

この値を使って塗りを決める代表的な規則が、 **non-zero（非ゼロ）** と **even-odd（偶奇）** です。

| 点の巻き数 | non-zero | even-odd |
| ---------: | :------: | :------: |
|          0 | 塗らない | 塗らない |
|    +1 / −1 |   塗る   |   塗る   |
|    +2 / −2 |   塗る   | 塗らない |

non-zero は、符号を含めた合計が 0 でなければ塗ります。even-odd は、巻き数の絶対値が奇数なら塗り、偶数なら塗りません。「輪郭線を横切る回数が奇数か偶数か」と考えるとわかりやすいでしょう。

## ピクセルを「点」ではなく小さな面として見る

輪郭線がピクセルの内部を横切ると、そのピクセルには輪郭線の内側と外側が同時に含まれることがあります。それでもピクセルの中心だけを見て「内側か外側か」を一度で決めると、隣り合うピクセルの塗りが急に切り替わり、斜め線や曲線がギザギザに見えます。

そこで、ピクセルを小さな四角形として考えます。四角形の中のそれぞれの場所で巻き数を調べ、その平均を取ります。輪郭線の内側の面積がピクセルの半分なら、おおよそ半分の濃さで塗ります。この「ピクセルのうち形が覆う割合」を **カバレッジ（coverage、被覆率）** と呼びます。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 620 260" role="img" aria-labelledby="windfoil-pixel-title windfoil-pixel-desc">
  <title id="windfoil-pixel-title">曲線がピクセルを覆う割合とカバレッジ</title>
  <desc id="windfoil-pixel-desc">曲線で区切られた青い領域がピクセル四角形のおよそ半分を覆う。平均カバレッジをおよそ50パーセントとして塗る。</desc>
  <rect width="620" height="260" rx="12" fill="#f8fafc"/>
  <text x="50" y="34" font-size="17" fill="#334155">ピクセルの中で、巻き数がある場所を平均する</text>
  <rect x="80" y="62" width="230" height="160" fill="#fff" stroke="#111827" stroke-width="3"/>
  <path d="M80 152 C145 132 208 132 310 152 L310 62 L80 62 Z" fill="#60a5fa" opacity=".9"/>
  <path d="M80 152 C145 132 208 132 310 152" fill="none" stroke="#1d4ed8" stroke-width="4"/>
  <text x="102" y="100" font-size="16" fill="#1e3a8a">形の内側</text>
  <text x="102" y="207" font-size="16" fill="#334155">形の外側</text>
  <line x1="350" y1="142" x2="405" y2="142" stroke="#64748b" stroke-width="2"/>
  <path d="M397 135 L409 142 L397 149" fill="none" stroke="#64748b" stroke-width="2"/>
  <rect x="430" y="82" width="120" height="120" rx="8" fill="#dbeafe" stroke="#94a3b8" stroke-width="2"/>
  <rect x="430" y="82" width="120" height="120" rx="8" fill="#2563eb" opacity=".5"/>
  <text x="433" y="226" font-size="16" fill="#334155">約 50% のカバレッジ</text>
</svg>

「平均を取る」と言うと、ピクセル内の無数の点を実際に数えるように聞こえるかもしれません。Windfoil の狙いは、点を大量に試すことではありません。輪郭線の式から、ピクセルの中の平均を **計算式で直接求める（解析積分する）** ことです。

## 積分を、面積の足し算として考える

積分という言葉は難しく見えますが、ここでは「とても細かく分けた値を足し合わせて平均すること」と考えてください。曲線がピクセルの中で作る細い帯を、幅の小さな短冊に分けて足し合わせるイメージです。

ピクセルの場所ごとの巻き数を \(w(x,y)\) と書くと、そのピクセルで平均した巻き数は次のように表せます。

```text
ピクセル内の平均巻き数
  = ピクセルの全域での巻き数の合計 ÷ ピクセルの面積
```

数式に慣れている人向けに書けば、ピクセル領域を \(P\)、その面積を \(|P|\) として、

```text
平均巻き数 = (1 / |P|) ∫P w(x, y) dA
```

です。`∫` は「領域全体にわたって足し合わせる」という記号、`dA` は「小さな面積」のことです。式を知らなくても、**ピクセル内で巻き数を平均する**という意味がわかれば十分です。

では、なぜピクセルの中を調べる代わりに輪郭線を使えるのでしょうか。面積と境界線の積分を結びつける数学の定理（グリーンの定理）を使うと、面全体の巻き数の合計を、輪郭線に沿った計算に置き換えられます。直感的な説明は[グリーンの定理の補足](#補足グリーンの定理を直感でつかむ)にあります。Windfoil では、曲線がピクセルの左端より左にあれば寄与はなく、右端より右にあればその高さの分だけピクセル全幅を寄与し、ピクセル内を通るところだけが部分的な寄与になります。この三つを曲線に沿って足し合わせれば、ピクセル内の平均巻き数を求められます。

### 平均した巻き数から、塗る割合へ

輪郭線が単純な形なら、平均巻き数の絶対値はカバレッジとしてそのまま使えます。たとえば、ピクセルの 30% が巻き数 `+1` の領域で、残りが 0 なら平均は `+0.3` です。逆向きの輪郭線なら `−0.3` ですが、塗る濃さはどちらも 30% です。

実装では次のように扱います。

- **non-zero**: 平均巻き数の絶対値を 0〜1 の範囲に収める。符号が逆でも同じ濃さにする。
- **even-odd**: 巻き数の絶対値を 2 周期で折り返し、偶数回の領域は塗らず、奇数回の領域を塗る。

この「折り返し」も、まずは単純な例で見てみましょう。

| 平均巻き数の絶対値 | non-zero の濃さ | even-odd の濃さ |
| -----------------: | --------------: | --------------: |
|                0.0 |              0% |              0% |
|                0.5 |             50% |             50% |
|                1.0 |            100% |            100% |
|                1.5 |            100% |             50% |
|                2.0 |            100% |              0% |

塗りの結果は輪郭線の向きをひっくり返しても変わりません。巻き数はプラスからマイナスに反転しますが、塗り分けは絶対値と偶奇に基づいているからです。これをここでは**カバレッジの向きに対する対称性**と呼びます。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 620 260" role="img" aria-labelledby="windfoil-symmetry-title windfoil-symmetry-desc">
  <title id="windfoil-symmetry-title">輪郭線を逆向きにしてもカバレッジは同じ</title>
  <desc id="windfoil-symmetry-desc">左では輪郭線の向きが時計回りでピクセルの平均巻き数がプラス0.4、右では逆向きでマイナス0.4。どちらも絶対値は0.4なので同じ40パーセントのカバレッジになる。</desc>
  <defs>
    <marker id="sym-arrow-blue" markerWidth="9" markerHeight="8" refX="7" refY="4" orient="auto">
      <path d="M0,0 L8,4 L0,8 Z" fill="#1d4ed8"/>
    </marker>
    <marker id="sym-arrow-red" markerWidth="9" markerHeight="8" refX="7" refY="4" orient="auto">
      <path d="M0,0 L8,4 L0,8 Z" fill="#dc2626"/>
    </marker>
  </defs>
  <rect width="620" height="260" rx="12" fill="#f8fafc"/>
  <text x="125" y="34" font-size="17" fill="#334155">輪郭線 A</text>
  <text x="422" y="34" font-size="17" fill="#334155">逆向きの輪郭線 B</text>
  <rect x="76" y="60" width="180" height="120" fill="#fff" stroke="#94a3b8" stroke-width="2"/>
  <rect x="76" y="60" width="180" height="120" fill="#2563eb" opacity=".4"/>
  <path d="M105 145 C105 78 218 78 218 145 C218 165 164 168 105 145" fill="none" stroke="#1d4ed8" stroke-width="4" marker-end="url(#sym-arrow-blue)"/>
  <rect x="362" y="60" width="180" height="120" fill="#fff" stroke="#94a3b8" stroke-width="2"/>
  <rect x="362" y="60" width="180" height="120" fill="#2563eb" opacity=".4"/>
  <path d="M391 145 C450 168 504 165 504 145 C504 78 391 78 391 145" fill="none" stroke="#dc2626" stroke-width="4" marker-end="url(#sym-arrow-red)"/>
  <text x="111" y="211" font-size="17" fill="#1e3a8a">平均巻き数 +0.4</text>
  <text x="398" y="211" font-size="17" fill="#991b1b">平均巻き数 −0.4</text>
  <text x="192" y="242" font-size="16" fill="#334155">どちらも |平均巻き数| = 0.4 → 同じ 40% の濃さ</text>
</svg>

積分そのものは、ピクセル内の**平均巻き数**を求めます。平均巻き数を塗り規則に合わせてカバレッジに変換した結果が、ピクセル内の真の塗り面積と一致するのは、ピクセル内で巻き数が 0 と隣り合う一つのレベル（`+1` または `−1`）だけを行き来する場合です。これは普通の輪郭の縁で成り立つ条件です。

一方、ピクセル内で正負の巻き数が打ち消し合う場合、絶対値が 1 より大きい場合、または even-odd で離れた偶奇レベルをまたぐ場合には、平均という一つの値だけから真の塗り面積を復元できません。通常描画では平均巻き数を塗り規則に合わせてカバレッジへ**換算**するため、このような複雑な重なりでは面積の近似になります。ここで「換算」は平均巻き数をそのままカバレッジと見なすという意味ではありません。non-zero では絶対値を 1 で頭打ちにし、even-odd では値を周期 2 の三角波に沿って折り返します。通常の輪郭の縁では正確さを保ちつつ、複雑な重なりで残る誤差は受け入れる設計です。つまり**平均巻き数の積分は正確でも、そこからカバレッジへ換算する段階には既知の限界があります**。

## 曲線を式で扱える形に整える

フォントの輪郭線には、直線だけでなく滑らかな曲線も含まれます。炊紙では輪郭線を**二次ベジェ曲線**の列として扱います。二次ベジェ曲線は、始点、制御点、終点の 3 点で形を決める曲線です。直線も、制御点を中間に置いた二次曲線として同じ形式で扱えます。

両バックエンドを共存させる実装では、従来の頂点・インデックスデータを保持し、`VectorVertex::curves()` で輪郭辺を二次ベジェ曲線に復元します。塗りつぶし用の補助三角形は除き、直線と曲線の向きや閉じ線を保ちます。これにより既存の字形キャッシュ形式は変更せず、保存済みのデータも両方式で利用できます。

Windfoil 用の GPU 曲線バッファは初めて必要になったときに生成します。従来の GPU バッファも保持するため、方式を戻す際にフォントを再変換する必要はありません。一方で、Windfoil を使った後は両方式のバッファ分のメモリを使います。

曲線の一部分が上がったり下がったりと折り返すと、どこで輪郭線と交わったかを調べるのが難しくなります。そこで、曲線の x 方向または y 方向が途中で折り返す場所（極値）で分割し、各部分がそれぞれの軸について単調に増えるか、単調に減るようにします。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 620 250" role="img" aria-labelledby="windfoil-split-title windfoil-split-desc">
  <title id="windfoil-split-title">折り返す曲線を単調な部分に分割する</title>
  <desc id="windfoil-split-desc">一つの二次ベジェ曲線がy方向に折り返す頂点で分割され、二つの単調な区間になる。</desc>
  <rect width="620" height="250" rx="12" fill="#f8fafc"/>
  <path d="M80 190 Q300 10 540 190" fill="none" stroke="#cbd5e1" stroke-width="3" stroke-dasharray="7 5"/>
  <path d="M80 190 Q190 100 310 100" fill="none" stroke="#2563eb" stroke-width="5"/>
  <path d="M310 100 Q420 100 540 190" fill="none" stroke="#7c3aed" stroke-width="5"/>
  <line x1="310" y1="100" x2="310" y2="220" stroke="#94a3b8" stroke-width="2" stroke-dasharray="6 5"/>
  <circle cx="310" cy="100" r="6" fill="#dc2626"/>
  <text x="322" y="91" font-size="15" fill="#991b1b">折り返し（極値）</text>
  <text x="96" y="224" font-size="16" fill="#1e3a8a">単調な区間 1</text>
  <text x="394" y="224" font-size="16" fill="#5b21b6">単調な区間 2</text>
  <text x="80" y="35" font-size="16" fill="#334155">一つの曲線を極値で分けると、各区間は一方向に進む</text>
</svg>

単調な区間にすれば、たとえば「この区間は y がこの高さを通るか」「通るなら曲線上のどの位置か」を一意に求めやすくなります。GPU 側では、二次曲線の式を使ってその位置を計算します。

## 水平バンドで、調べる曲線を減らす

すべてのピクセルについて、図形中のすべての曲線を調べるのは無駄が多くなります。あるピクセルの高さと関係しない曲線は、そのピクセルの塗りに影響しないからです。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 680 310" role="img" aria-labelledby="windfoil-band-filter-title windfoil-band-filter-desc">
  <title id="windfoil-band-filter-title">ピクセルの高さに応じて調べる曲線を絞る</title>
  <desc id="windfoil-band-filter-desc">中央のピクセル四角形と高さが重なる青い曲線は積分の候補になる。ピクセルより上または下にある灰色の曲線は高さが重ならないので、このピクセルの積分では調べない。</desc>
  <rect width="680" height="310" rx="12" fill="#f8fafc"/>
  <text x="24" y="32" font-size="16" fill="#334155">このピクセルの高さ範囲にある曲線だけを候補にする</text>
  <rect x="225" y="95" width="220" height="120" fill="#fff" stroke="#111827" stroke-width="3"/>
  <path d="M35 66 C95 38 146 86 202 66 M470 66 C525 40 580 84 642 59" fill="none" stroke="#94a3b8" stroke-width="4" stroke-dasharray="8 6"/>
  <path d="M42 250 C96 218 154 269 204 242 M470 250 C528 220 582 270 640 239" fill="none" stroke="#94a3b8" stroke-width="4" stroke-dasharray="8 6"/>
  <path d="M70 185 C150 170 200 112 282 125 C353 137 408 177 485 119 C535 82 582 111 615 126" fill="none" stroke="#2563eb" stroke-width="5"/>
  <text x="28" y="116" font-size="15" fill="#475569">上の曲線：高さが重ならない</text>
  <line x1="183" y1="111" x2="210" y2="72" stroke="#64748b" stroke-width="2"/>
  <text x="28" y="292" font-size="15" fill="#475569">下の曲線：高さが重ならない</text>
  <line x1="190" y1="276" x2="211" y2="246" stroke="#64748b" stroke-width="2"/>
  <text x="250" y="156" font-size="15" fill="#1e3a8a">ピクセル</text>
  <text x="462" y="185" font-size="15" fill="#1d4ed8">高さが重なるので</text>
  <text x="462" y="205" font-size="15" fill="#1d4ed8">積分の候補になる</text>
</svg>

Windfoil は輪郭線の高さを複数の **水平バンド** に分け、各バンドと交差する曲線だけを登録します。描画時にはピクセルが含まれるバンドを選び、その中の曲線を調べます。さらに、曲線を右端の位置が大きい順に並べておき、ピクセルより左にある曲線に達したら探索を打ち切れるようにします。

バンドは計算対象を絞るための索引です。バンドの境界をまたぐピクセルでは、ピクセル全体を一つのバンドに押し込めず、境界で分けてそれぞれの部分を積分します。これにより境界での塗り漏れを避けます。

## 変形や斜めの視点では、描画経路が変わる

ここまで説明した水平バンドの経路は、輪郭線がローカル座標上でそのまま描かれる場合のものです。炊紙では、文字の位置・回転・カメラ変換に加えて、文字ごとのモーションも使います。特に、変形量が曲線上の位置に応じて変わるモーションでは、輪郭線の座標自体を変形してから画面へ投影する必要があります。

そのたびに各ピクセルが全制御点を変形・投影すると、同じ曲線を何度も処理することになります。そこで炊紙では、距離に応じた変形を使うインスタンスについて、描画の前に compute シェーダーで始点・制御点・終点を投影し、結果を GPU の storage buffer に保存します。その後、ピクセルシェーダーは保存済みの点を読み、画面座標で曲線の折り返しとピクセルへの寄与を計算します。静止した文字やこの種の変形を使わない文字には、この前処理を行いません。

この経路では、前処理した曲線を元の水平バンドに分類し直すのではなく、各ピクセルが元の曲線列を調べます。そのため、制御点の投影をピクセルごとに繰り返すコストは避けられますが、曲線数や描画ピクセル数が多いと積分自体の負荷は残ります。通常の水平バンド経路と同じ速さになるとは限りません。

真横に近い視点では、文字が画面上で非常に細くなります。局所座標上でアンチエイリアス用の余白を広げると、投影がつぶれるほど余白も大きくなり、実際の輪郭より広い範囲を描こうとしてしまいます。そこで描画範囲は、投影した輪郭の画面上の境界から求め、near / far 平面で切り、画面の内側に制限してから、アンチエイリアス用に約 1 ピクセルだけ広げます。投影した幅または高さが 1 ピクセル未満になる場合は、局所座標での積分が不安定になりやすいため、画面座標上で積分します。

通常経路では、ピクセルの画面座標から文字平面上の局所座標を逆変換して、曲線積分に使います。斜めから見るとこの変換は細長く引き伸ばされます。回転やせん断を含む場合、局所座標で積分する矩形は画面上のピクセルの形に対する近似です。曲線の解析積分そのものと、座標変換を含めた最終的なカバレッジの精度は区別する必要があります。

透視投影を行うと、二次ベジェ曲線は一般に有理曲線になります。現在の変形経路では始点・制御点・終点をそれぞれ投影して二次曲線として扱うため、透視変換された曲線の近似です。特に極端な透視や極細の輪郭では、精度と計算量に注意が必要です。

## 1 ピクセルを描くまで

静止した文字や距離依存モーションを使わない文字は、次の通常経路で描画します。

1. 輪郭線を二次ベジェ曲線の列にする。直線も同じ曲線形式にそろえる。
2. x または y の折り返し位置で曲線を単調な部分に分ける。
3. 曲線を水平バンドに分類し、バンドごとに右端の順に並べる。
4. 描画するピクセルに関係するバンドと曲線を選ぶ。
5. ピクセルの四角形の中で、各曲線が作る巻き数の寄与を解析的に積分する。
6. 平均巻き数を non-zero または even-odd の規則でカバレッジに変換し、色とアルファ値に反映する。

距離依存モーションでは、単調分割と水平バンドの経路の代わりに、compute シェーダーで始点・制御点・終点を変形・投影してから、保存した画面座標の曲線を使って積分します。どちらの経路でも、描画範囲を画面上で求めて near / far 平面で切り、アンチエイリアス用の余白を加えます。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 450" role="img" aria-labelledby="windfoil-pipeline-title windfoil-pipeline-desc">
  <title id="windfoil-pipeline-title">Windfoil が1ピクセルを描くまでの6段階</title>
  <desc id="windfoil-pipeline-desc">輪郭線を二次ベジェ曲線にそろえ、曲線を単調分割して水平バンドに登録し、対象ピクセルの候補曲線を選択する。候補から平均巻き数を積分し、塗り規則でカバレッジと色に変換する6段階の図。</desc>
  <defs>
    <marker id="pipeline-arrow" markerWidth="9" markerHeight="8" refX="7" refY="4" orient="auto">
      <path d="M0,0 L8,4 L0,8 Z" fill="#64748b"/>
    </marker>
  </defs>
  <rect width="760" height="450" rx="14" fill="#f8fafc"/>
  <g fill="#fff" stroke="#cbd5e1" stroke-width="2">
    <rect x="20" y="26" width="220" height="174" rx="12"/>
    <rect x="270" y="26" width="220" height="174" rx="12"/>
    <rect x="520" y="26" width="220" height="174" rx="12"/>
    <rect x="20" y="240" width="220" height="174" rx="12"/>
    <rect x="270" y="240" width="220" height="174" rx="12"/>
    <rect x="520" y="240" width="220" height="174" rx="12"/>
  </g>
  <g stroke="#64748b" stroke-width="2.5" fill="none" marker-end="url(#pipeline-arrow)">
    <line x1="242" y1="112" x2="263" y2="112"/>
    <line x1="492" y1="112" x2="513" y2="112"/>
    <path d="M630 202 L630 222 L130 222 L130 232"/>
    <line x1="242" y1="326" x2="263" y2="326"/>
    <line x1="492" y1="326" x2="513" y2="326"/>
  </g>
  <g font-family="sans-serif" fill="#0f172a">
    <g font-size="16" font-weight="700" fill="#1d4ed8">
      <text x="36" y="54">1  曲線にそろえる</text>
      <text x="286" y="54">2  折り返しで分割</text>
      <text x="536" y="54">3  水平バンドに登録</text>
      <text x="36" y="268">4  候補を選ぶ</text>
      <text x="286" y="268">5  巻き数を積分</text>
      <text x="536" y="268">6  色と濃さにする</text>
    </g>
    <g font-size="13" fill="#475569">
      <text x="32" y="184">全ての輪郭を二次ベジェ曲線で表す</text>
      <text x="286" y="184">x・yの極値で一方向の区間へ</text>
      <text x="536" y="184">高さごとに曲線を整理する</text>
      <text x="36" y="398">ピクセルと高さが重なる線だけ</text>
      <text x="286" y="398">ピクセル内の平均巻き数を計算</text>
      <text x="536" y="398">塗り規則でcoverageへ変換</text>
    </g>
  </g>
  <path d="M55 142 C78 90 116 90 139 142 C160 185 198 175 214 120" fill="none" stroke="#2563eb" stroke-width="4"/>
  <circle cx="55" cy="142" r="4" fill="#1d4ed8"/>
  <circle cx="214" cy="120" r="4" fill="#1d4ed8"/>
  <circle cx="119" cy="111" r="4" fill="#dc2626"/>
  <path d="M300 137 Q380 62 460 137" fill="none" stroke="#cbd5e1" stroke-width="3" stroke-dasharray="5 4"/>
  <path d="M300 137 Q340 99.5 380 99.5" fill="none" stroke="#2563eb" stroke-width="4"/>
  <path d="M380 99.5 Q420 99.5 460 137" fill="none" stroke="#7c3aed" stroke-width="4"/>
  <line x1="380" y1="99.5" x2="380" y2="148" stroke="#94a3b8" stroke-width="1.5" stroke-dasharray="4 4"/>
  <circle cx="380" cy="99.5" r="4" fill="#dc2626"/>
  <text x="289" y="158" font-size="11" fill="#1e3a8a">単調な区間 1</text>
  <text x="390" y="158" font-size="11" fill="#5b21b6">単調な区間 2</text>
  <text x="338" y="176" font-size="11" fill="#991b1b">極値で分割</text>
  <g fill="#e2e8f0">
    <rect x="545" y="77" width="170" height="24" rx="4"/>
    <rect x="545" y="107" width="170" height="24" rx="4"/>
    <rect x="545" y="137" width="170" height="24" rx="4"/>
  </g>
  <path d="M552 158 C585 145 610 82 653 86 C677 90 686 143 710 150" fill="none" stroke="#2563eb" stroke-width="4"/>
  <path d="M552 93 C590 78 620 80 648 91" fill="none" stroke="#94a3b8" stroke-width="3" stroke-dasharray="5 4"/>
  <rect x="586" y="103" width="46" height="31" fill="#fff" stroke="#dc2626" stroke-width="2"/>
  <rect x="36" y="302" width="178" height="58" fill="#fff" stroke="#94a3b8" stroke-width="2"/>
  <path d="M36 344 C83 335 119 312 214 321" fill="none" stroke="#2563eb" stroke-width="4"/>
  <rect x="100" y="302" width="43" height="58" fill="none" stroke="#dc2626" stroke-width="2" stroke-dasharray="4 3"/>
  <text x="38" y="377" font-size="12" fill="#dc2626">対象ピクセル</text>
  <rect x="290" y="300" width="180" height="61" fill="#fff" stroke="#94a3b8" stroke-width="2"/>
  <path d="M290 339 C340 331 374 305 470 317 L470 361 L290 361 Z" fill="#60a5fa" opacity=".8"/>
  <path d="M290 339 C340 331 374 305 470 317" fill="none" stroke="#1d4ed8" stroke-width="3"/>
  <text x="306" y="382" font-size="12" fill="#334155">平均巻き数 → coverage</text>
  <rect x="550" y="300" width="160" height="62" rx="5" fill="#dbeafe" stroke="#94a3b8" stroke-width="2"/>
  <rect x="550" y="300" width="160" height="62" rx="5" fill="#2563eb" opacity=".48"/>
  <text x="588" y="386" font-size="13" fill="#334155">例：半分ほど塗る</text>
</svg>

アンチエイリアスが無効な場合は、ピクセル内を積分せず、ピクセルの中心で巻き数を判定します。そのため結果は「中心点が塗りの内側なら塗る」という形になり、境界の滑らかさは得られません。

## Windfoil が目指すもの

Windfoil の特徴は、三角形をたくさん描いて重ねる代わりに、輪郭線そのものを使ってピクセルのカバレッジを求めることです。曲線に沿った細い形も、ピクセルの中でどの程度覆われるかを平均することで、滑らかな縁として描けます。

もちろん、積分を使えばすべての形で真の面積比が完全に得られる、という意味ではありません。複雑な重なりでの近似や、GPU の浮動小数点精度などの制約は残ります。また、曲線の数や座標変換によっては計算量が増えることもあります。それでも「輪郭線がどこを通るか」から被覆率を直接求める考え方は、ベクター形状を小さなピクセルへ変換する仕組みを理解するうえで、とても役立ちます。

## 実装を読む

炊紙での実装やテストを読む場合は、次のファイルが入口になります。リンク先は、両バックエンドが共存する実装のコミットに固定しています。

- [`font_rasterizer/src/rasterizer_renderrer.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/rasterizer_renderrer.rs): バックエンドの選択と、同じ GPU バッファを使った往復切り替えの回帰テスト。
- [`font_rasterizer/src/legacy_renderrer.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/legacy_renderrer.rs): 従来の三角形描画と輪郭抽出。
- [`font_rasterizer/src/windfoil_renderrer.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/windfoil_renderrer.rs): Windfoil の描画パイプライン、変形の前投影、色の合成。
- [`font_rasterizer/src/windfoil.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/windfoil.rs): 曲線の単調分割、水平バンドへの分類、GPU バッファへの登録。
- [`font_rasterizer/src/shader/windfoil.wgsl`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/shader/windfoil.wgsl): 通常経路の投影範囲・逆投影、変形後の画面範囲と巻き数積分、カバレッジへの変換。
- [`font_rasterizer/src/projected_curves.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/projected_curves.rs): 変形時に制御点を一度だけ投影して共有する compute pipeline とバッファ管理。
- [`font_rasterizer/src/vector_vertex.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/vector_vertex.rs): 従来の三角形生成と、輪郭辺から Windfoil 用の曲線を復元する処理。
- [`font_rasterizer/src/vector_vertex_png_renderer.rs`](https://github.com/mitoma/kashiki2/blob/59e1103a8c978c880ea19c09ebe2bf4ae5a58418/font_rasterizer/src/vector_vertex_png_renderer.rs): Windfoil の GPU 回帰テスト。

変形時の前投影を無効にした参照経路との描画結果の比較は、`windfoil_preprojection_matches_direct_deformation` テストで行えます。これは画像の一致を調べるもので、性能計測ではありません。compute 前処理を追加しても、変形後の曲線分割とピクセル積分は引き続き GPU 上で行われます。

GPU 回帰テストは WebGPU 対応 GPU が利用できる環境で、次のコマンドから実行できます。

```powershell
cargo test -p font_rasterizer windfoil_gpu_coverage_regressions --lib -- --ignored
cargo test -p font_rasterizer windfoil_preprojection_matches_direct_deformation --lib -- --ignored
cargo test -p font_rasterizer windfoil_grazing_view_stays_within_projected_bounds --lib -- --ignored
cargo test -p font_rasterizer rendering_backends_switch_with_shared_buffers --lib -- --ignored
```

## 補足：グリーンの定理を直感でつかむ

グリーンの定理は、ひとことで言うと、**ある面全体で足し合わせた量を、その面のふちを一周する計算に置き換えられる**という考え方です。Windfoil では、ピクセルの中を細かく区切って巻き数を調べるかわりに、ピクセルを囲む輪郭線が作る寄与を計算します。

### 床のタイルを数えるかわりに、ふちに沿って計算する

たとえば、床にでこぼこした形の敷物があるとします。敷物が床をどれだけ覆うかを知りたいとき、床全体を細かなタイルに分けて「敷物が乗っているタイル」を数える方法があります。しかし、タイルをどれほど小さくしても、境界にあるタイルは「全部敷物」か「一部だけ敷物」かを別に考える必要があります。

もう一つは、敷物の **ふちを一周たどりながら、各場所の位置と進む向きから決まる値を足し合わせる** 方法です。ここでいう「ふちに沿って計算する」は、線の長さを測ることではありません。ふちの位置と、上向き・下向きなど進む向きの両方を使って値を足します。たとえば面積を求めるとき、同じ長さの辺でも、形のどこにあるかやどちら向きに進むかで足し引きする量は変わります。グリーンの定理が示すのは、適切な足し方を選べば、「面の中で足し合わせた量」と「ふちに沿って足し合わせた量」が等しくなる、ということです。四角形の面積を頂点の座標から求める方法（靴ひも公式）も、この考え方の身近な例です。

### ピクセルでは何をたどるの？

Windfoil が知りたいのは、ピクセルの中にある巻き数の合計です。ここでグリーンの定理を使うと、その合計を、ピクセルの範囲に対する輪郭線の寄与の足し算に変換できます。ここでも曲線の長さを測るのではありません。曲線がどこを通り、その場所でどちら向きに進むかから、ピクセル内の巻き数合計への寄与を計算します。実際の計算は輪郭線の曲線ごとに行い、曲線がピクセルより左にある部分、右にある部分、ピクセルを横切る部分を区別します。

右側の曲線も無視できません。曲線がピクセルの右端より外にあれば、その高さの変化にピクセルの全幅を掛けた分が寄与します。ピクセル内を通る曲線では、ピクセルの左端から曲線までの横幅を、その曲線が上がる・下がる向きに応じて足し引きします。左端より左の部分は寄与しません。これらを曲線に沿って足し合わせると、ピクセル内の巻き数の合計になります。最後にピクセルの面積で割れば平均巻き数です。

つまり、グリーンの定理は「ピクセルの内側をサンプルしなくてもよい」理由を与え、Windfoil の曲線積分は、その考えを二次ベジェ曲線とピクセルの四角形に合わせて具体的に計算する部分です。グリーンの定理そのものの証明や微積分の知識がなくても、**線の長さではなく、位置と向きに応じた寄与をふちに沿って足す**という役割を押さえれば、アルゴリズムの流れを理解できます。

### 単調な二次ベジェ曲線をピクセル内で積分する

今度は、ピクセルの左下から右上へ向かって進む二次ベジェ曲線を考えます。この例では x と y の両方が一方向に増えるので、ピクセルのある高さを決めると、曲線上の場所は一つに定まります。

その高さにある細い横帯を見れば、曲線の左側でピクセル内に入っている幅がわかります。図の青い部分は、その幅を高さ方向に積み重ねた領域です。横帯の幅を高さに沿って足す、というのは考え方をつかむためのイメージです。Windfoil は横帯を何枚も実際に作って足すのではなく、この面積を曲線の式から計算します。曲線がピクセル内にある区間では「左端から曲線までの幅」が寄与し、曲線が右端より先に出た後はピクセル全幅が寄与します。左端より外にある区間の幅は 0 です。曲線の向きが逆なら、同じ寄与を逆符号で数えます。

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 680 340" role="img" aria-labelledby="windfoil-integral-curve-title windfoil-integral-curve-desc">
  <title id="windfoil-integral-curve-title">単調な二次ベジェ曲線を横帯の幅で積分する</title>
  <desc id="windfoil-integral-curve-desc">ピクセルの中を左下から右上へ単調に進む二次ベジェ曲線を描き、曲線の左側の青い面積を薄い水平帯に分けて示す。各帯で左端から曲線までの幅を求めて足し合わせる。</desc>
  <defs>
    <marker id="integral-arrow" markerWidth="9" markerHeight="8" refX="7" refY="4" orient="auto">
      <path d="M0,0 L8,4 L0,8 Z" fill="#334155"/>
    </marker>
  </defs>
  <rect width="680" height="340" rx="12" fill="#f8fafc"/>
  <text x="28" y="32" font-size="16" fill="#334155">各高さで「左端から曲線までの幅」を足し合わせる</text>
  <rect x="145" y="58" width="390" height="220" fill="#fff" stroke="#111827" stroke-width="3"/>
  <path d="M145 278 Q270 174 535 58 L145 58 Z" fill="#60a5fa" opacity=".64"/>
  <path d="M145 278 Q270 174 535 58" fill="none" stroke="#1d4ed8" stroke-width="5"/>
  <g stroke="#0ea5e9" stroke-width="2" opacity=".8">
    <line x1="145" y1="238" x2="225" y2="238"/>
    <line x1="145" y1="198" x2="311" y2="198"/>
    <line x1="145" y1="158" x2="397" y2="158"/>
    <line x1="145" y1="118" x2="480" y2="118"/>
  </g>
  <line x1="145" y1="198" x2="311" y2="198" stroke="#0f172a" stroke-width="3" marker-end="url(#integral-arrow)"/>
  <circle cx="145" cy="278" r="5" fill="#0f172a"/>
  <circle cx="535" cy="58" r="5" fill="#0f172a"/>
  <text x="151" y="299" font-size="14" fill="#334155">始点 t = 0</text>
  <text x="449" y="49" font-size="14" fill="#334155">終点 t = 1</text>
  <text x="205" y="187" font-size="14" fill="#0f172a">この高さの幅</text>
  <text x="342" y="230" font-size="16" fill="#1e3a8a">幅を積み重ねた寄与</text>
  <text x="552" y="117" font-size="14" fill="#334155">ピクセル上端</text>
  <text x="552" y="277" font-size="14" fill="#334155">ピクセル下端</text>
  <text x="26" y="322" font-size="13" fill="#475569">横帯を無数にサンプルするのではなく、曲線の式から合計を直接計算する</text>
</svg>

実際には横帯を何百枚も作って足すわけではありません。二次ベジェ曲線は、始点から終点までの位置を `t`（0 から 1 までの進み具合）で表す式です。まず曲線がピクセルの上端・下端に達する `t` を求め、横端をまたぐ場合は左端・右端に達する `t` でも区間を分けます。曲線が x と y の両方で単調なら、それぞれの境界を通る場所は一つなので、どの区間を計算すればよいかがはっきりします。

区間の中では、横幅も高さ方向の進み方も `t` の式で表せます。それらを掛けて足し合わせる計算は、二次曲線なら有限回の計算で（理想的な計算精度では）正確に求められます。これが「解析的に積分する」という意味です。つまり、細かな点をたくさん試す近似ではなく、曲線がピクセルを横切る位置を式で見つけ、区間ごとの寄与を式で合計します。

#### 小さな数値例

計算の形を見てみましょう。説明のため、幅も高さも 1 のピクセルを考えます。始点を左下 `(0, 0)`、制御点を `(1/4, 3/4)`、終点を右上 `(1, 1)` に置いた二次ベジェ曲線にします。この曲線は x と y がどちらも増え続けます。

`t` が 0 から 1 へ進むとき、曲線の位置は次の式になります。

```text
x(t) = 1/2 t + 1/2 t²
y(t) = 3/2 t − 1/2 t²
```

この曲線の左側にある青い面積は、各高さでの横幅を足したものです。微積分の記法では「横幅 x(t) に、高さ方向の小さな進み dy を掛けて、t=0 から t=1 まで足す」と書けます。

```text
面積 = ∫ x(t) dy
     = ∫₀¹ x(t) × y'(t) dt
     = ∫₀¹ (1/2 t + 1/2 t²)(3/2 − t) dt
     = 1/3
```

つまりこの例では、1×1 のピクセルのうち 3 分の 1 が曲線の左側です。式の積分記号を追わなくても、「高さごとの幅を全部足した面積が 1/3」という読み方で構いません。実際の Windfoil は同じ考え方を曲線がピクセルの端をまたぐ場合にも適用し、境界で区間を分けて計算します。こうして得られるのはまずピクセル内の平均巻き数であり、複雑な重なりでのカバレッジ変換には前述の制約が残ります。
