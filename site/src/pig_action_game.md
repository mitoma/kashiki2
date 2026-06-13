# Pig Action Game

font_rasterizer を使った小さなアクションゲームです。Web 版では iframe 内の canvas にフォーカスした状態で操作してください。

- ← ↑ ↓ → : 移動
- クリックまたはタッチ : 移動
- Space : ジャンプ
- ピンチアウト : フルスクリーン
- ピンチイン : フルスクリーン解除

<style>
.pig-action-game {
    display: grid;
    gap: 0px;
    margin: 0px;
    padding: 0px;
    width: 1024px;
    height: 768px;
    border: none;
}
</style>

<iframe
    class="pig-action-game"
    src="./pig_action_game_embed.html"
    title="Pig Action Game"
    loading="lazy"
    scrolling="no"
></iframe>
