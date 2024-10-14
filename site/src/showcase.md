# ShowCase

Technical showcase.
This demo use WASM & WebGL.

Click area and edit it!
<div id="kashikishi-area"></div>

<script type="module">
  import init from "./wasm/showcase/showcase.js";
  init().then(() => {
    console.log("WASM Loaded");
  });
</script>
