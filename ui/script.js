// Use the global Tauri API
const invoke = window.__TAURI__.core.invoke;

// Application state
let currentSection = 'review';
let currentReviewCards = [];
let currentCardIndex = 0;
let currentCard = null;
let lastDeletedCard = null; // Store last deleted card for undo

document.addEventListener('DOMContentLoaded', () => {
    initializeApp();
});

async function initializeApp() {
    setupNavigation();
    setupEventListeners();
    await loadReviewStats();
    await loadCards();
}

function setupNavigation() {
    const navButtons = document.querySelectorAll('.nav-btn');
    console.log('Found navigation buttons:', navButtons.length);

    navButtons.forEach(btn => {
        btn.addEventListener('click', (e) => {
            const sectionName = e.target.id.replace('nav-', '');
            console.log('Navigation button clicked:', sectionName);
            showSection(sectionName);
        });
    });
}

function showSection(sectionName) {
    // Update navigation - use both selectors to be safe
    document.querySelectorAll('.nav-btn, [id^="nav-"]').forEach(btn => {
        btn.classList.remove('bg-emerald-600', 'hover:bg-emerald-700', 'ring-2', 'ring-emerald-400/30', 'shadow-lg', 'shadow-emerald-600/25');
        btn.classList.add('bg-zinc-700', 'hover:bg-zinc-600', 'hover:shadow-lg');
    });

    const activeBtn = document.getElementById(`nav-${sectionName}`);
    if (activeBtn) {
        activeBtn.classList.remove('bg-zinc-700', 'hover:bg-zinc-600', 'hover:shadow-lg');
        activeBtn.classList.add('bg-emerald-600', 'hover:bg-emerald-700', 'ring-2', 'ring-emerald-400/30', 'shadow-lg', 'shadow-emerald-600/25');
    }

    // Update sections
    document.querySelectorAll('.section').forEach(section => {
        section.classList.add('hidden');
    });

    const targetSection = document.getElementById(`${sectionName}-section`);
    if (targetSection) {
        targetSection.classList.remove('hidden');
    }

    currentSection = sectionName;

    // Load section-specific data
    if (sectionName === 'browse') {
        loadCards();
    } else if (sectionName === 'stats') {
        loadDetailedStats();
    } else if (sectionName === 'review') {
        loadReviewStats();
    }
} function setupEventListeners() {
    // Review section
    document.getElementById('start-review').addEventListener('click', startReview);
    document.getElementById('show-answer-btn').addEventListener('click', showAnswer);

    // Rating buttons
    const ratingButtons = document.querySelectorAll('.rating-btn');
    console.log('Found rating buttons:', ratingButtons.length);

    ratingButtons.forEach(btn => {
        btn.addEventListener('click', (e) => {
            const difficulty = parseInt(e.target.dataset.difficulty);
            console.log('Rating button clicked with difficulty:', difficulty);
            rateCard(difficulty);
        });
    });

    // Create card form
    document.getElementById('create-card-form').addEventListener('submit', createCard);

    // Browse cards
    document.getElementById('refresh-cards').addEventListener('click', loadCards);

    // Event delegation for delete buttons (since they're created dynamically)
    document.addEventListener('click', (e) => {
        if (e.target.closest('.delete-card-btn')) {
            e.preventDefault();
            e.stopPropagation();
            const deleteBtn = e.target.closest('.delete-card-btn');
            const cardId = deleteBtn.dataset.cardId;
            console.log('Delete button clicked for card:', cardId);
            if (cardId) {
                deleteCard(cardId);
            } else {
                console.error('No card ID found on delete button');
            }
        }
    });
}

async function loadReviewStats() {
    try {
        const stats = await invoke('get_review_stats');
        document.getElementById('cards-due').textContent = stats.cards_due;
        document.getElementById('total-cards').textContent = stats.total_cards;

        // Update start button
        const startBtn = document.getElementById('start-review');
        if (stats.cards_due > 0) {
            startBtn.textContent = `Review ${stats.cards_due} Cards`;
            startBtn.disabled = false;
            startBtn.classList.remove('opacity-50', 'cursor-not-allowed');
        } else {
            startBtn.textContent = 'No Cards Due';
            startBtn.disabled = true;
            startBtn.classList.add('opacity-50', 'cursor-not-allowed');
        }
    } catch (error) {
        console.error('Failed to load review stats:', error);
        showError('Failed to load review statistics');
    }
}

async function startReview() {
    try {
        const dueCards = await invoke('get_due_cards');

        if (dueCards.length === 0) {
            showError('No cards are due for review');
            return;
        }

        currentReviewCards = dueCards;
        currentCardIndex = 0;

        // Hide start button, show card interface
        document.getElementById('start-review').parentElement.classList.add('hidden');
        document.getElementById('review-card').classList.remove('hidden');

        showCurrentCard();
    } catch (error) {
        console.error('Failed to start review:', error);
        showError('Failed to start review session');
    }
}

function showCurrentCard() {
    if (currentCardIndex >= currentReviewCards.length) {
        finishReview();
        return;
    }

    currentCard = currentReviewCards[currentCardIndex];

    // Update progress
    document.getElementById('current-card-num').textContent = currentCardIndex + 1;
    document.getElementById('total-review-cards').textContent = currentReviewCards.length;
    const progress = ((currentCardIndex) / currentReviewCards.length) * 100;
    document.getElementById('review-progress').style.width = `${progress}%`;

    // Show card front
    document.getElementById('card-front-text').textContent = currentCard.front;
    document.getElementById('card-front').classList.remove('hidden');
    document.getElementById('card-back').classList.add('hidden');
    document.getElementById('show-answer-btn').classList.remove('hidden');
    document.getElementById('rating-buttons').classList.add('hidden');
}

function showAnswer() {
    document.getElementById('card-back-text').textContent = currentCard.back;
    document.getElementById('card-back').classList.remove('hidden');
    document.getElementById('show-answer-btn').classList.add('hidden');
    document.getElementById('rating-buttons').classList.remove('hidden');
}

async function rateCard(difficulty) {
    console.log('Rating card with difficulty:', difficulty, 'Card ID:', currentCard?.id);

    try {
        const result = await invoke('review_card', {
            id: currentCard.id,
            difficulty: difficulty
        });

        console.log('Rating successful, result:', result);

        currentCardIndex++;
        showCurrentCard();
    } catch (error) {
        console.error('Failed to rate card:', error);
        showError('Failed to save card rating');
    }
}

function finishReview() {
    // Hide card interface, show completion message
    document.getElementById('review-card').classList.add('hidden');
    document.getElementById('start-review').parentElement.classList.remove('hidden');

    showSuccess('Review session completed!');
    loadReviewStats();
}

async function createCard(e) {
    e.preventDefault();

    const front = document.getElementById('card-front-input').value.trim();
    const back = document.getElementById('card-back-input').value.trim();
    const category = document.getElementById('card-category-input').value.trim() || null;

    if (!front || !back) {
        showError('Both front and back are required');
        return;
    }

    try {
        await invoke('create_card', {
            request: {
                front: front,
                back: back,
                category: category
            }
        });

        // Clear form
        document.getElementById('create-card-form').reset();
        showSuccess('Card created successfully!');

        // Update stats
        await loadReviewStats();

    } catch (error) {
        console.error('Failed to create card:', error);
        showError('Failed to create card');
    }
}

async function loadCards() {
    console.log('=== LOADING CARDS ===');
    try {
        const cards = await invoke('get_cards');
        console.log('Loaded cards from backend:', cards.length, 'cards');
        console.log('Card details:', cards);

        const cardsList = document.getElementById('cards-list');

        if (cards.length === 0) {
            cardsList.innerHTML = '<p class="text-zinc-400 text-center py-8">No cards yet. Create your first card!</p>';
            return;
        }

        cardsList.innerHTML = cards.map(card => {
            console.log('Creating HTML for card:', card.id);
            return `
            <div class="bg-zinc-800/50 rounded-lg p-4 border border-zinc-700">
                <div class="flex justify-between items-start mb-2">
                    <div class="flex-1">
                        <div class="font-medium mb-1">${escapeHtml(card.front)}</div>
                        <div class="text-sm text-zinc-400 mb-2">${escapeHtml(card.back)}</div>
                        ${card.category ? `<span class="inline-block bg-zinc-700 text-xs px-2 py-1 rounded">${escapeHtml(card.category)}</span>` : ''}
                    </div>
                    <button data-card-id="${card.id}" class="delete-card-btn text-red-400 hover:text-red-300 ml-2 p-1 rounded hover:bg-red-400/10 transition-colors">
                        🗑️
                    </button>
                </div>
                <div class="text-xs text-zinc-500 mt-2">
                    Reviews: ${card.review_count} | Interval: ${card.interval} days
                    ${card.next_review ? `| Next: ${new Date(card.next_review).toLocaleDateString()}` : ''}
                </div>
            </div>`;
        }).join('');

        // Verify delete buttons were created
        const deleteButtons = document.querySelectorAll('.delete-card-btn');
        console.log('Delete buttons found after creation:', deleteButtons.length);
        deleteButtons.forEach((btn, index) => {
            console.log(`Delete button ${index}:`, btn.dataset.cardId);
        });

        console.log('Cards loaded, delete buttons should work via event delegation');

    } catch (error) {
        console.error('Failed to load cards:', error);
        showError('Failed to load cards');
    }
}

async function deleteCard(cardId) {
    console.log('=== DELETE CARD FUNCTION CALLED ===');
    console.log('deleteCard called with cardId:', cardId);

    if (!cardId) {
        console.error('No card ID provided to deleteCard');
        showError('Invalid card ID');
        return;
    }

    console.log('Deleting card immediately:', cardId);

    try {
        // First, get the card data before deleting (for undo functionality)
        const cardToDelete = await invoke('get_card', { id: cardId });
        console.log('Retrieved card for deletion:', cardToDelete);

        if (!cardToDelete) {
            showError('Card not found');
            return;
        }

        // Store for undo
        lastDeletedCard = cardToDelete;

        console.log('Calling Tauri delete_card command...');
        await invoke('delete_card', { id: cardId });
        console.log('Card deleted successfully, refreshing UI...');

        await loadCards();
        console.log('Cards reloaded');

        await loadReviewStats();
        console.log('Stats reloaded');

        // Show success message with undo option
        showSuccessWithUndo('Card deleted');
        console.log('Success message with undo shown');
    } catch (error) {
        console.error('Failed to delete card - error details:', error);
        showError('Failed to delete card: ' + error);
    }
    console.log('=== DELETE CARD FUNCTION COMPLETED ===');
} async function loadDetailedStats() {
    try {
        const stats = await invoke('get_review_stats');

        document.getElementById('stat-total').textContent = stats.total_cards;
        document.getElementById('stat-due').textContent = stats.cards_due;
        document.getElementById('stat-new').textContent = stats.cards_new;
        document.getElementById('stat-mature').textContent = stats.cards_mature;

    } catch (error) {
        console.error('Failed to load detailed stats:', error);
        showError('Failed to load statistics');
    }
}

function showSuccess(message) {
    const successEl = document.getElementById('success-message');
    successEl.textContent = message;
    successEl.classList.remove('hidden');
    setTimeout(() => {
        successEl.classList.add('hidden');
    }, 3000);
}

function showSuccessWithUndo(message) {
    const successEl = document.getElementById('success-message');
    successEl.innerHTML = `
        ${message} 
        <button onclick="undoDelete()" 
                class="ml-2 px-3 py-1 bg-white/20 hover:bg-white/30 rounded text-sm transition-colors">
            Undo
        </button>
    `;
    successEl.classList.remove('hidden');

    // Auto-hide after 8 seconds (longer for undo)
    setTimeout(() => {
        successEl.classList.add('hidden');
    }, 8000);
}

async function undoDelete() {
    if (!lastDeletedCard) {
        showError('No card to restore');
        return;
    }

    try {
        console.log('Undoing delete for card:', lastDeletedCard.front);

        // Recreate the card (it will get a new ID, but that's fine)
        await invoke('create_card', {
            request: {
                front: lastDeletedCard.front,
                back: lastDeletedCard.back,
                category: lastDeletedCard.category
            }
        });

        await loadCards();
        await loadReviewStats();

        // Hide the undo message
        document.getElementById('success-message').classList.add('hidden');

        showSuccess('Card restored successfully');
        console.log('Card restored successfully');

        // Clear the stored card
        lastDeletedCard = null;
    } catch (error) {
        console.error('Failed to restore card:', error);
        showError('Failed to restore card');
    }
}

function showError(message) {
    const errorEl = document.getElementById('error-message');
    errorEl.textContent = message;
    errorEl.classList.remove('hidden');
    setTimeout(() => {
        errorEl.classList.add('hidden');
    }, 3000);
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Make functions globally available for onclick handlers
window.undoDelete = undoDelete;
