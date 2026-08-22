const modal = document.querySelector('#modal');
const toast = document.querySelector('#toast');
const walletButton = document.querySelector('#walletButton');
const walletLabel = document.querySelector('#walletLabel');
const recipient = document.querySelector('#recipient');

function showToast(message) {
  toast.textContent = message;
  toast.classList.add('show');
  window.setTimeout(() => toast.classList.remove('show'), 3200);
}

function openModal() {
  modal.hidden = false;
  recipient.focus();
}

function closeModal() {
  modal.hidden = true;
}

document.querySelector('#mintButton').addEventListener('click', openModal);
document.querySelector('#closeModal').addEventListener('click', closeModal);
modal.addEventListener('click', (event) => {
  if (event.target === modal) closeModal();
});
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') closeModal();
});

walletButton.addEventListener('click', () => {
  walletLabel.textContent = 'G...A7K9';
  walletButton.querySelector('.wallet-dot').style.background = '#c8f169';
  showToast('Demo wallet connected.');
});

document.querySelector('#confirmMint').addEventListener('click', () => {
  if (!recipient.value.trim()) {
    recipient.focus();
    showToast('Enter a recipient address first.');
    return;
  }
  closeModal();
  showToast('Demo only: mint_ticket is ready for wallet signing.');
});

document.querySelector('#filterButton').addEventListener('click', () => {
  showToast('Showing all tickets.');
});

document.querySelectorAll('.nav-link').forEach((link) => {
  link.addEventListener('click', () => {
    document.querySelectorAll('.nav-link').forEach((item) => item.classList.remove('active'));
    link.classList.add('active');
  });
});
