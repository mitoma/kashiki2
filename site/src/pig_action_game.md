# Pig Action Game

font_rasterizer を使った小さなアクションゲームです。Web 版では iframe 内の canvas にフォーカスした状態で操作してください。

- ← ↑ ↓ → : 移動
- クリックまたはタッチ : 移動
- Space : ジャンプ
- テレビをタッチ : フルスクリーン切り替え

<script>
document.documentElement.style.setProperty(
  "--device-pixel-ratio",
  window.devicePixelRatio
);
</script>

<style>
.pig-action-game {
    display: grid;
    gap: 0px;
    margin: 0px;
    padding: 0px;
    width: calc(1024px / var(--device-pixel-ratio));
    height: calc(768px / var(--device-pixel-ratio));
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
