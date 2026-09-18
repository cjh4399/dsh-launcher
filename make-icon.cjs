// 一次性脚本：把微信图片转成 1024x1024 正方形 PNG 作为应用图标源
const sharp = require("sharp");
const path = require("path");

const src = path.join(__dirname, "app-icon-src.jpg");
const dst = path.join(__dirname, "app-icon.png");

sharp(src)
  .resize(1024, 1024, { fit: "cover", position: "centre" })
  .png()
  .toFile(dst)
  .then((info) => console.log("OK", info.width + "x" + info.height, info.size + "B"))
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
