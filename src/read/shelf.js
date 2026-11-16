document.querySelectorAll('.book img').forEach((image) => {
  const drop = () => image.remove();

  if (image.complete && image.naturalWidth === 0) {
    drop();
    return;
  }

  image.addEventListener('error', drop, { once: true });
});
