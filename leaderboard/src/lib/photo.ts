const MAX_SIDE = 256;
const QUALITY = 0.82;

function extensionOf(name) {
  const match = /\.([a-z0-9]+)$/i.exec(name ?? "");
  return match ? match[1].toLowerCase() : "";
}

export function readImageAsSquareDataUrl(file) {
  return new Promise((resolve, reject) => {
    const extension = extensionOf(file.name);

    if (!file.type.startsWith("image/") && !["heic", "heif"].includes(extension)) {
      reject(new Error("that file is not an image"));
      return;
    }

    const reader = new FileReader();
    reader.onerror = () => reject(new Error("could not read the file"));
    reader.onload = () => {
      const image = new Image();
      image.onerror = () => {
        const hint = ["heic", "heif"].includes(extension)
          ? "this browser cannot open HEIC photos, export it as JPEG or PNG first"
          : "could not decode that image, try a JPEG or PNG";
        reject(new Error(hint));
      };
      image.onload = () => {
        const side = Math.min(image.width, image.height);
        if (!side) {
          reject(new Error("that image has no pixels"));
          return;
        }
        const canvas = document.createElement("canvas");
        canvas.width = MAX_SIDE;
        canvas.height = MAX_SIDE;
        const ctx = canvas.getContext("2d");
        ctx.drawImage(
          image,
          (image.width - side) / 2,
          (image.height - side) / 2,
          side,
          side,
          0,
          0,
          MAX_SIDE,
          MAX_SIDE,
        );
        resolve(canvas.toDataURL("image/jpeg", QUALITY));
      };
      image.src = reader.result as string;
    };
    reader.readAsDataURL(file);
  });
}
