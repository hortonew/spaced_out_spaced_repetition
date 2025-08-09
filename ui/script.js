// Use the global Tauri API
const invoke = window.__TAURI__.core.invoke;

// Configuration
const CONFIG = {
    // Notification timeouts (in milliseconds)
    SUCCESS_TIMEOUT: 1200,    // Regular success messages
    ERROR_TIMEOUT: 3000,      // Error messages  
    UNDO_TIMEOUT: 8000        // Messages with undo option
};

// Application state
let currentSection = 'review';
let currentReviewCards = [];
let currentCardIndex = 0;
let currentCard = null;
let lastDeletedCard = null; // Store last deleted card for undo

// Notification state
let notificationCountdowns = {
    success: null,
    error: null
};

// Organization state
let allCards = []; // Cache of all cards for filtering
let selectedCards = new Set(); // Selected card IDs for bulk operations
let tags = []; // Available tags

document.addEventListener('DOMContentLoaded', () => {
    initializeApp();
});

async function initializeApp() {
    setupNavigation();
    setupEventListeners();
    await loadReviewStats();
    await loadCards();

    // Always start on the review section
    showSection('review');
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

    // Save the current section to localStorage
    localStorage.setItem('currentSection', sectionName);

    // Load section-specific data
    if (sectionName === 'browse') {
        loadCards();
        loadTags();
    } else if (sectionName === 'tags') {
        loadTagStats();
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

    // Organization features
    document.getElementById('search-input').addEventListener('input', debounce(filterCards, 300));
    document.getElementById('tag-filter').addEventListener('change', filterCards);
    document.getElementById('select-all').addEventListener('change', toggleSelectAll);
    document.getElementById('bulk-actions-btn').addEventListener('click', toggleBulkMode);
    document.getElementById('bulk-delete-btn').addEventListener('click', bulkDeleteCards);
    document.getElementById('bulk-tag-select').addEventListener('change', bulkUpdateTag);

    // Event delegation for delete buttons and card selection (since they're created dynamically)
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

    // Event delegation for card selection checkboxes
    document.addEventListener('change', (e) => {
        if (e.target.classList.contains('card-checkbox')) {
            const cardId = e.target.dataset.cardId;
            if (e.target.checked) {
                selectedCards.add(cardId);
            } else {
                selectedCards.delete(cardId);
            }
            updateSelectionControls();
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
    const tag = document.getElementById('card-tag-input').value.trim() || null;

    if (!front || !back) {
        showError('Both front and back are required');
        return;
    }

    try {
        await invoke('create_card', {
            request: {
                front: front,
                back: back,
                tag: tag
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
        allCards = cards; // Cache for filtering
        displayCards(cards);
    } catch (error) {
        console.error('Error loading cards:', error);
        showNotification('Failed to load cards', 'error');
    }
}

function displayCards(cards) {
    const cardsList = document.getElementById('cards-list');
    const bulkActionsBtn = document.getElementById('bulk-actions-btn');

    if (cards.length === 0) {
        cardsList.innerHTML = '<p class="text-zinc-400 text-center py-8">No cards found.</p>';
        bulkActionsBtn.classList.add('hidden');
        return;
    }

    bulkActionsBtn.classList.remove('hidden');

    cardsList.innerHTML = cards.map(card => {
        const isSelected = selectedCards.has(card.id);
        return `
        <div class="bg-zinc-800/50 rounded-lg p-4 border border-zinc-700">
            <div class="flex items-start space-x-3">
                <input type="checkbox" class="card-checkbox mt-1 rounded bg-zinc-700 border-zinc-600 text-emerald-600 focus:ring-emerald-500" 
                       data-card-id="${card.id}" ${isSelected ? 'checked' : ''}>
                <div class="flex-1">
                    <div class="font-medium mb-1">${escapeHtml(card.front)}</div>
                    <div class="text-sm text-zinc-400 mb-2">${escapeHtml(card.back)}</div>
                    ${card.tag ? `<span class="inline-block bg-zinc-700 text-xs px-2 py-1 rounded">${escapeHtml(card.tag)}</span>` : ''}
                </div>
                <button data-card-id="${card.id}" class="delete-card-btn text-red-400 hover:text-red-300 p-1 rounded hover:bg-red-400/10 transition-colors">
                    🗑️
                </button>
            </div>
            <div class="text-xs text-zinc-500 mt-2">
                Reviews: ${card.review_count} | Interval: ${card.interval} days
                ${card.next_review ? `| Next: ${new Date(card.next_review).toLocaleDateString()}` : ''}
            </div>
        </div>`;
    }).join('');

    updateSelectionControls();
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
    const timeout = CONFIG.SUCCESS_TIMEOUT;
    let timeLeft = timeout / 1000; // Convert to seconds with decimals

    // Clear any existing countdown
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
    }

    // Create the message structure once with countdown
    successEl.innerHTML = `
        <div class="flex items-center justify-between">
            <span>${message}</span>
            <span id="countdown-timer" class="text-sm opacity-75 ml-3">${timeLeft.toFixed(1)}s</span>
        </div>
    `;

    successEl.classList.remove('hidden');

    // Get reference to the countdown timer element
    const countdownEl = document.getElementById('countdown-timer');

    // Update only the countdown text every 100ms for smooth decimal countdown
    notificationCountdowns.success = setInterval(() => {
        timeLeft -= 0.1;
        if (timeLeft > 0 && countdownEl) {
            countdownEl.textContent = `${timeLeft.toFixed(1)}s`;
        } else {
            clearInterval(notificationCountdowns.success);
            notificationCountdowns.success = null;
        }
    }, 100);

    setTimeout(() => {
        successEl.classList.add('hidden');
        if (notificationCountdowns.success) {
            clearInterval(notificationCountdowns.success);
            notificationCountdowns.success = null;
        }
    }, timeout);
}

function showSuccessWithUndo(message) {
    const successEl = document.getElementById('success-message');
    const timeout = CONFIG.UNDO_TIMEOUT;
    let timeLeft = timeout / 1000; // Convert to seconds with decimals

    // Clear any existing countdown
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
    }

    // Create the message structure once with countdown and buttons
    successEl.innerHTML = `
        <div class="flex items-center justify-between">
            <span>${message}</span>
            <div class="flex items-center space-x-2">
                <span id="countdown-timer" class="text-sm opacity-75">${timeLeft.toFixed(1)}s</span>
                <button onclick="undoDelete()" 
                        class="px-3 py-2 bg-white/20 hover:bg-white/30 rounded text-sm transition-colors font-medium">
                    Undo
                </button>
                <button onclick="dismissUndo()" 
                        class="px-3 py-2 bg-zinc-600/50 hover:bg-zinc-500/50 rounded text-sm transition-colors">
                    Dismiss
                </button>
            </div>
        </div>
    `;

    successEl.classList.remove('hidden');

    // Get reference to the countdown timer element
    const countdownEl = document.getElementById('countdown-timer');

    // Update only the countdown text every 100ms for smooth decimal countdown
    notificationCountdowns.success = setInterval(() => {
        timeLeft -= 0.1;
        if (timeLeft > 0 && countdownEl) {
            countdownEl.textContent = `${timeLeft.toFixed(1)}s`;
        } else {
            clearInterval(notificationCountdowns.success);
            notificationCountdowns.success = null;
        }
    }, 100);

    // Auto-hide after configured time (longer for undo)
    setTimeout(() => {
        successEl.classList.add('hidden');
        if (notificationCountdowns.success) {
            clearInterval(notificationCountdowns.success);
            notificationCountdowns.success = null;
        }
    }, timeout);
}

async function undoDelete() {
    if (!lastDeletedCard) {
        showError('No card to restore');
        return;
    }

    // Clear the countdown and hide the undo message immediately
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
        notificationCountdowns.success = null;
    }
    document.getElementById('success-message').classList.add('hidden');

    try {
        console.log('Undoing delete for card:', lastDeletedCard.front);

        // Recreate the card (it will get a new ID, but that's fine)
        await invoke('create_card', {
            request: {
                front: lastDeletedCard.front,
                back: lastDeletedCard.back,
                tag: lastDeletedCard.tag
            }
        });

        await loadCards();
        await loadReviewStats();

        showSuccess('Card restored successfully');
        console.log('Card restored successfully');

        // Clear the stored card
        lastDeletedCard = null;
    } catch (error) {
        console.error('Failed to restore card:', error);
        showError('Failed to restore card');
    }
}

function dismissUndo() {
    // Clear the countdown and hide the undo message immediately
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
        notificationCountdowns.success = null;
    }
    document.getElementById('success-message').classList.add('hidden');

    // Clear the stored card since user dismissed the undo option
    lastDeletedCard = null;
}

function showError(message) {
    const errorEl = document.getElementById('error-message');
    const timeout = CONFIG.ERROR_TIMEOUT;
    let timeLeft = timeout / 1000; // Convert to seconds with decimals

    // Clear any existing countdown
    if (notificationCountdowns.error) {
        clearInterval(notificationCountdowns.error);
    }

    // Create the message structure once with countdown
    errorEl.innerHTML = `
        <div class="flex items-center justify-between">
            <span>${message}</span>
            <span id="error-countdown-timer" class="text-sm opacity-75 ml-3">${timeLeft.toFixed(1)}s</span>
        </div>
    `;

    errorEl.classList.remove('hidden');

    // Get reference to the countdown timer element
    const countdownEl = document.getElementById('error-countdown-timer');

    // Update only the countdown text every 100ms for smooth decimal countdown
    notificationCountdowns.error = setInterval(() => {
        timeLeft -= 0.1;
        if (timeLeft > 0 && countdownEl) {
            countdownEl.textContent = `${timeLeft.toFixed(1)}s`;
        } else {
            clearInterval(notificationCountdowns.error);
            notificationCountdowns.error = null;
        }
    }, 100);

    setTimeout(() => {
        errorEl.classList.add('hidden');
        if (notificationCountdowns.error) {
            clearInterval(notificationCountdowns.error);
            notificationCountdowns.error = null;
        }
    }, timeout);
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Make functions globally available for onclick handlers
window.undoDelete = undoDelete;
window.dismissUndo = dismissUndo;

// ============================================================================
// ORGANIZATION FEATURES
// ============================================================================

// Load tags for dropdowns
async function loadTags() {
    try {
        tags = await invoke('get_tags');
        updateTagDropdowns();
    } catch (error) {
        console.error('Failed to load tags:', error);
    }
}

function updateTagDropdowns() {
    const tagFilter = document.getElementById('tag-filter');
    const bulkTagSelect = document.getElementById('bulk-tag-select');

    // Update tag filter
    tagFilter.innerHTML = '<option value="">All Tags</option>';
    tags.forEach(tag => {
        tagFilter.innerHTML += `<option value="${escapeHtml(tag)}">${escapeHtml(tag)}</option>`;
    });

    // Update bulk tag select
    bulkTagSelect.innerHTML = '<option value="">Change Tag</option>';
    tags.forEach(tag => {
        bulkTagSelect.innerHTML += `<option value="${escapeHtml(tag)}">${escapeHtml(tag)}</option>`;
    });
}

// Search and filter cards
async function filterCards() {
    const searchQuery = document.getElementById('search-input').value.trim();
    const tagFilter = document.getElementById('tag-filter').value;

    try {
        const searchRequest = {
            query: searchQuery || null,
            tag: tagFilter || null,
            tags: null
        };

        const filteredCards = await invoke('search_cards', { request: searchRequest });
        displayCards(filteredCards);
    } catch (error) {
        console.error('Failed to filter cards:', error);
        showNotification('Failed to filter cards', 'error');
    }
}

// Debounce function for search input
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}

// Bulk operations
function toggleBulkMode() {
    const selectionControls = document.getElementById('selection-controls');
    const isVisible = !selectionControls.classList.contains('hidden');

    if (isVisible) {
        // Hide bulk mode
        selectionControls.classList.add('hidden');
        selectedCards.clear();
        displayCards(allCards);
    } else {
        // Show bulk mode
        selectionControls.classList.remove('hidden');
    }
}

function toggleSelectAll() {
    const selectAllCheckbox = document.getElementById('select-all');
    const cardCheckboxes = document.querySelectorAll('.card-checkbox');

    if (selectAllCheckbox.checked) {
        cardCheckboxes.forEach(checkbox => {
            checkbox.checked = true;
            selectedCards.add(checkbox.dataset.cardId);
        });
    } else {
        cardCheckboxes.forEach(checkbox => {
            checkbox.checked = false;
            selectedCards.delete(checkbox.dataset.cardId);
        });
    }

    updateSelectionControls();
}

function updateSelectionControls() {
    const selectedCount = document.getElementById('selected-count');
    const selectAllCheckbox = document.getElementById('select-all');
    const bulkDeleteBtn = document.getElementById('bulk-delete-btn');

    selectedCount.textContent = `${selectedCards.size} selected`;

    // Update select all checkbox state
    const cardCheckboxes = document.querySelectorAll('.card-checkbox');
    const checkedBoxes = document.querySelectorAll('.card-checkbox:checked');

    if (checkedBoxes.length === 0) {
        selectAllCheckbox.indeterminate = false;
        selectAllCheckbox.checked = false;
    } else if (checkedBoxes.length === cardCheckboxes.length) {
        selectAllCheckbox.indeterminate = false;
        selectAllCheckbox.checked = true;
    } else {
        selectAllCheckbox.indeterminate = true;
    }

    // Enable/disable bulk actions
    bulkDeleteBtn.disabled = selectedCards.size === 0;
}

async function bulkDeleteCards() {
    if (selectedCards.size === 0) return;

    const cardIds = Array.from(selectedCards);

    try {
        await invoke('delete_multiple_cards', { cardIds });
        showNotification(`Deleted ${cardIds.length} cards successfully`, 'success');
        selectedCards.clear();
        await loadCards();
        await loadReviewStats();
    } catch (error) {
        console.error('Failed to delete cards:', error);
        showNotification('Failed to delete cards', 'error');
    }
}

async function bulkUpdateTag() {
    const bulkTagSelect = document.getElementById('bulk-tag-select');
    const newTag = bulkTagSelect.value;

    if (!newTag || selectedCards.size === 0) return;

    const cardIds = Array.from(selectedCards);

    try {
        const request = {
            card_ids: cardIds,
            tag: newTag
        };

        await invoke('bulk_update_tag', { request });
        showNotification(`Updated tag for ${cardIds.length} cards`, 'success');
        selectedCards.clear();
        await loadCards();
        bulkTagSelect.value = '';
    } catch (error) {
        console.error('Failed to update tag:', error);
        showNotification('Failed to update tag', 'error');
    }
}

// Tag statistics
async function loadTagStats() {
    try {
        const tagStats = await invoke('get_tag_stats');
        displayTagStats(tagStats);
    } catch (error) {
        console.error('Failed to load tag stats:', error);
        showNotification('Failed to load tag statistics', 'error');
    }
}

function displayTagStats(tagStats) {
    const tagStatsList = document.getElementById('tag-stats-list');

    if (tagStats.length === 0) {
        tagStatsList.innerHTML = '<p class="text-zinc-400 text-center py-8">No tags found.</p>';
        return;
    }

    tagStatsList.innerHTML = tagStats.map(stats => `
        <div class="bg-zinc-800/50 rounded-lg p-4 border border-zinc-700">
            <div class="flex justify-between items-start mb-4">
                <h3 class="text-lg font-medium">${escapeHtml(stats.name)}</h3>
                <span class="text-sm text-zinc-400">${stats.total_cards} cards</span>
            </div>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div class="text-center">
                    <div class="text-xl font-bold text-emerald-400">${stats.cards_due}</div>
                    <div class="text-xs text-zinc-400">Due</div>
                </div>
                <div class="text-center">
                    <div class="text-xl font-bold text-blue-400">${stats.cards_new}</div>
                    <div class="text-xs text-zinc-400">New</div>
                </div>
                <div class="text-center">
                    <div class="text-xl font-bold text-yellow-400">${stats.total_cards - stats.cards_new - stats.cards_mature}</div>
                    <div class="text-xs text-zinc-400">Learning</div>
                </div>
                <div class="text-center">
                    <div class="text-xl font-bold text-purple-400">${stats.cards_mature}</div>
                    <div class="text-xs text-zinc-400">Mature</div>
                </div>
            </div>
        </div>
    `).join('');
}
