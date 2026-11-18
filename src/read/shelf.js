let offline = {};

try {
  offline = JSON.parse(localStorage.getItem('kobink:offline')) || {};
} catch {}

document.querySelectorAll('.book').forEach((book) => {
  const id = (book.getAttribute('href') || '').split('/').pop();

  if (!offline[id]) return;

  const mark = document.createElement('span');
  mark.className = 'offline-mark';
  mark.textContent = 'Offline';
  book.querySelector('.cover')?.append(mark);
});

document.querySelectorAll('.book img').forEach((image) => {
  const drop = () => image.remove();

  if (image.complete && image.naturalWidth === 0) {
    drop();
    return;
  }

  image.addEventListener('error', drop, { once: true });
});
