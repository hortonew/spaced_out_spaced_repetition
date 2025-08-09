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
let lastDeletedCards = []; // Store multiple deleted cards for bulk undo

// Edit mode state
let editMode = false;
let editingCardId = null;
let editingFromReview = false; // Track if editing from review session

// Notification state
let notificationCountdowns = {
    success: null,
    error: null
};

let notificationTimeouts = {
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
    // Set up window size for desktop platforms
    await setupWindowSize();

    setupNavigation();
    setupEventListeners();
    await loadReviewStats();
    await loadCards();

    // Always start on the review section
    showSection('review');
}

async function setupWindowSize() {
    try {
        // Check if we're running in Tauri (desktop) environment
        if (window.__TAURI__ && window.__TAURI__.window) {
            const { appWindow } = window.__TAURI__.window;
            const { availableMonitors, currentMonitor } = window.__TAURI__.window;

            // Get current monitor information
            const monitor = await currentMonitor();
            if (monitor) {
                // Calculate 80% of screen height, but keep reasonable width
                const targetHeight = Math.floor(monitor.size.height * 0.8);
                const targetWidth = Math.min(1200, Math.floor(monitor.size.width * 0.7)); // Max 1200px wide or 70% of screen width

                // Set the window size
                await appWindow.setSize({
                    width: targetWidth,
                    height: targetHeight
                });

                // Center the window
                await appWindow.center();

                console.log(`Window resized to ${targetWidth}x${targetHeight} (80% of screen height)`);
            }
        }
    } catch (error) {
        // If window resizing fails, just continue - this is non-critical
        console.log('Window resizing not available or failed:', error);
    }
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

function resetBulkActions() {
    const selectionControls = document.getElementById('selection-controls');
    const bulkActionsBtn = document.getElementById('bulk-actions-btn');
    const bulkInstructions = document.getElementById('bulk-instructions');

    // Exit bulk mode if it's currently active and elements exist
    if (selectionControls && bulkActionsBtn && !selectionControls.classList.contains('hidden')) {
        selectionControls.classList.add('hidden');
        if (bulkInstructions) {
            bulkInstructions.classList.add('hidden');
        }
        selectedCards.clear();
        bulkActionsBtn.textContent = 'Bulk Actions';
        bulkActionsBtn.classList.remove('bg-orange-600', 'hover:bg-orange-700');
        bulkActionsBtn.classList.add('bg-blue-600', 'hover:bg-blue-700');
        console.log('Bulk mode reset when switching sections');
    }
}

function showSection(sectionName) {
    // Reset bulk actions when switching away from browse section
    if (currentSection === 'browse' && sectionName !== 'browse') {
        resetBulkActions();
    }

    // Reset edit state if navigating away from create section without completing edit
    if (currentSection === 'create' && sectionName !== 'create' && editMode) {
        // Only reset if user explicitly navigates away (not programmatically)
        // This preserves the edit flow when returning from edit to review
        if (sectionName === 'review' && editingFromReview) {
            // Allow return to review from edit - don't reset state
        } else {
            // User navigated away from edit - reset state
            editMode = false;
            editingCardId = null;
            editingFromReview = false;

            // Reset UI
            document.getElementById('create-section-title').textContent = 'Create New Card';
            document.getElementById('create-card-submit').textContent = 'Create Card';
            document.getElementById('cancel-edit-btn').classList.add('hidden');
            document.getElementById('create-card-form').reset();
        }
    }

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
    } else if (sectionName === 'settings') {
        showSettingsMenu();
    }
}

function setupEventListeners() {
    // Review section
    document.getElementById('start-review').addEventListener('click', startReview);
    document.getElementById('show-answer-btn').addEventListener('click', showAnswer);
    document.getElementById('review-edit-btn').addEventListener('click', editCurrentReviewCard);
    document.getElementById('review-delete-btn').addEventListener('click', deleteCurrentReviewCard);

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

    // Cancel edit button
    document.getElementById('cancel-edit-btn').addEventListener('click', cancelEdit);

    // Browse cards
    document.getElementById('refresh-cards').addEventListener('click', loadCards);

    // Organization features
    document.getElementById('search-input').addEventListener('input', debounce(filterCards, 300));
    document.getElementById('tag-filter').addEventListener('change', filterCards);
    document.getElementById('select-all').addEventListener('change', toggleSelectAll);
    document.getElementById('bulk-actions-btn').addEventListener('click', toggleBulkMode);
    document.getElementById('bulk-delete-btn').addEventListener('click', bulkDeleteCards);
    document.getElementById('bulk-tag-apply').addEventListener('click', bulkUpdateTag);

    // Handle tag input/select interaction
    document.getElementById('bulk-tag-select').addEventListener('change', (e) => {
        if (e.target.value) {
            document.getElementById('bulk-tag-input').value = e.target.value;
        }
    });

    document.getElementById('bulk-tag-input').addEventListener('input', (e) => {
        if (e.target.value) {
            document.getElementById('bulk-tag-select').value = '';
        }
    });

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
        } else if (e.target.closest('.edit-card-btn')) {
            e.preventDefault();
            e.stopPropagation();
            const editBtn = e.target.closest('.edit-card-btn');
            const cardId = editBtn.dataset.cardId;
            console.log('Edit button clicked for card:', cardId);
            if (cardId) {
                editCard(cardId);
            } else {
                console.error('No card ID found on edit button');
            }
        }
    });

    // Event delegation for card selection checkboxes
    document.addEventListener('change', (e) => {
        if (e.target.classList.contains('card-checkbox')) {
            const cardId = e.target.dataset.cardId;
            console.log('Card checkbox changed:', cardId, 'checked:', e.target.checked);
            if (e.target.checked) {
                selectedCards.add(cardId);
            } else {
                selectedCards.delete(cardId);
            }
            console.log('selectedCards now contains:', Array.from(selectedCards));
            updateSelectionControls();
        }
    });

    // Mobile keyboard handling
    setupMobileKeyboardHandling();
}

function setupMobileKeyboardHandling() {
    // Check if we're on a mobile device
    const isMobile = /Android|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);

    if (!isMobile) return;

    let originalViewportHeight = window.innerHeight;
    let isKeyboardVisible = false;
    let keyboardCheckInterval = null;

    // Function to scroll to top of current section
    function scrollToTopOfSection() {
        console.log('scrollToTopOfSection called');

        // Try multiple approaches for maximum reliability
        const currentSectionEl = document.querySelector('.section:not(.hidden)');
        const createSection = document.getElementById('create-section');
        const mainElement = document.querySelector('main');

        // Strategy 1: Scroll the current visible section
        if (currentSectionEl) {
            console.log('Scrolling current section:', currentSectionEl.id);
            currentSectionEl.scrollIntoView({
                behavior: 'smooth',
                block: 'start',
                inline: 'nearest'
            });
        }

        // Strategy 2: If we're in create section, scroll it specifically
        if (createSection && !createSection.classList.contains('hidden')) {
            console.log('Scrolling create section specifically');
            createSection.scrollIntoView({
                behavior: 'smooth',
                block: 'start',
                inline: 'nearest'
            });
        }

        // Strategy 3: Scroll main container
        if (mainElement) {
            console.log('Scrolling main element');
            mainElement.scrollIntoView({
                behavior: 'smooth',
                block: 'start',
                inline: 'nearest'
            });
        }

        // Strategy 4: Force scroll to absolute top
        console.log('Force scrolling to top of window');
        window.scrollTo({
            top: 0,
            behavior: 'smooth'
        });

        // Strategy 5: Immediate fallback for mobile
        setTimeout(() => {
            window.scrollTo({
                top: 0,
                behavior: 'auto'
            });
        }, 100);
    }

    // Make the scroll function globally available
    window.scrollToTopOfSection = scrollToTopOfSection;

    // Detect keyboard visibility changes
    function checkKeyboardVisibility() {
        const currentHeight = window.innerHeight;
        const heightDifference = originalViewportHeight - currentHeight;

        // Adjust keyboard threshold based on orientation
        const isLandscape = window.innerWidth > window.innerHeight;
        const keyboardThreshold = isLandscape ? 100 : 150; // Lower threshold for landscape

        const newKeyboardState = heightDifference > keyboardThreshold;

        if (newKeyboardState !== isKeyboardVisible) {
            isKeyboardVisible = newKeyboardState;
            console.log('Keyboard visibility changed:', isKeyboardVisible ? 'visible' : 'hidden',
                'Landscape:', isLandscape, 'Threshold:', keyboardThreshold);

            // Handle landscape mode keyboard visibility
            if (isLandscape) {
                handleLandscapeKeyboard(newKeyboardState);
            }

            if (!isKeyboardVisible) {
                // Keyboard just disappeared - scroll to top after a brief delay
                setTimeout(() => {
                    const createSection = document.getElementById('create-section');
                    if (createSection && !createSection.classList.contains('hidden')) {
                        scrollToTopOfSection();
                    }
                }, 200);
            }
        }
    }

    // Special handling for landscape mode keyboard
    function handleLandscapeKeyboard(keyboardVisible) {
        const createSection = document.getElementById('create-section');
        const form = document.getElementById('create-card-form');
        const body = document.body;
        const main = document.querySelector('main');

        if (!createSection || !form) return;

        if (keyboardVisible) {
            console.log('Keyboard visible in landscape - ensuring scrollable viewport');

            // Add keyboard-visible class for CSS styling
            document.body.classList.add('keyboard-visible');

            // Force extra height to ensure scrollability - make the entire viewport much taller
            const extraHeight = window.innerHeight * 2.25; // 2.25x viewport height (reduced from 3x)
            body.style.minHeight = extraHeight + 'px';
            if (main) main.style.minHeight = extraHeight + 'px';
            form.style.minHeight = extraHeight + 'px';
            createSection.style.minHeight = extraHeight + 'px';

            // Ensure background extends all the way down
            createSection.style.background = 'rgb(9, 9, 11)'; // Match body background

            console.log('Set landscape heights to:', extraHeight + 'px');

            // Ensure we can scroll to see all form fields
            setTimeout(() => {
                const activeElement = document.activeElement;
                if (activeElement && (activeElement.tagName === 'INPUT' || activeElement.tagName === 'TEXTAREA')) {
                    // Force scroll to ensure the active input is visible
                    activeElement.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start',
                        inline: 'nearest'
                    });
                }
            }, 300);

        } else {
            console.log('Keyboard hidden in landscape - restoring normal heights');

            // Remove keyboard-visible class
            document.body.classList.remove('keyboard-visible');

            // Reset heights
            body.style.minHeight = '';
            if (main) main.style.minHeight = '';
            form.style.minHeight = '';
            createSection.style.minHeight = '';
            createSection.style.background = '';
        }
    }

    // Start monitoring keyboard visibility
    function startKeyboardMonitoring() {
        if (keyboardCheckInterval) return;

        keyboardCheckInterval = setInterval(checkKeyboardVisibility, 200);
        console.log('Started keyboard monitoring');
    }

    // Stop monitoring keyboard visibility
    function stopKeyboardMonitoring() {
        if (keyboardCheckInterval) {
            clearInterval(keyboardCheckInterval);
            keyboardCheckInterval = null;
            console.log('Stopped keyboard monitoring');
        }
    }

    // Simple and effective input focus handling
    function setupInputFocusHandling() {
        const inputs = document.querySelectorAll('#create-card-form input, #create-card-form textarea');

        inputs.forEach(input => {
            // Remove any existing listeners to avoid duplicates
            input.removeEventListener('focus', handleInputFocus);
            input.removeEventListener('input', handleInputChange);
            input.removeEventListener('blur', handleInputBlur);

            // Add focus listener
            input.addEventListener('focus', handleInputFocus);
            input.addEventListener('input', handleInputChange);
            input.addEventListener('blur', handleInputBlur);
        });
    }

    function handleInputFocus(e) {
        const input = e.target;
        console.log('Input focused:', input.id);

        // Start monitoring for keyboard hide events
        startKeyboardMonitoring();

        // Check if we're in landscape mode for more aggressive scrolling
        const isLandscape = window.innerWidth > window.innerHeight;

        // Special handling for tag input in landscape mode
        if (isLandscape && input.id === 'card-tag-input') {
            console.log('Tag input focused in landscape - using aggressive scrolling');

            // Force immediate scroll to top
            setTimeout(() => {
                window.scrollTo({
                    top: 0,
                    behavior: 'smooth'
                });
            }, 50);

            // Then scroll the form to ensure tag field is visible
            setTimeout(() => {
                const form = document.getElementById('create-card-form');
                if (form) {
                    form.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start',
                        inline: 'nearest'
                    });
                }
            }, 200);

            // Finally ensure the tag input itself is in view
            setTimeout(() => {
                input.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start',
                    inline: 'nearest'
                });
            }, 400);

            return; // Skip the normal handling for tag input in landscape
        }

        // Multiple strategies to ensure the input is visible
        setTimeout(() => {
            // Strategy 1: Scroll the input into view
            input.scrollIntoView({
                behavior: 'smooth',
                block: isLandscape ? 'start' : 'center', // More aggressive in landscape
                inline: 'nearest'
            });
        }, 100);

        setTimeout(() => {
            // Strategy 2: Scroll to top of the form
            const form = document.getElementById('create-card-form');
            if (form) {
                form.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start',
                    inline: 'nearest'
                });
            }
        }, isLandscape ? 200 : 300); // Faster in landscape

        setTimeout(() => {
            // Strategy 3: Force scroll the entire page to show the input
            const inputRect = input.getBoundingClientRect();
            const viewportHeight = window.innerHeight;

            // More aggressive scrolling in landscape mode
            const targetPosition = isLandscape ? viewportHeight / 6 : viewportHeight / 4;

            // If input is in bottom half of screen, scroll it to target position
            if (inputRect.top > viewportHeight / 2) {
                const scrollAmount = inputRect.top - targetPosition;
                window.scrollBy({
                    top: scrollAmount,
                    behavior: 'smooth'
                });
            }
        }, isLandscape ? 300 : 500); // Faster in landscape
    }

    function handleInputChange(e) {
        // Keep the input visible while typing
        const input = e.target;
        const inputRect = input.getBoundingClientRect();
        const viewportHeight = window.innerHeight;

        // If input is too low on screen, scroll up
        if (inputRect.bottom > viewportHeight - 50) {
            input.scrollIntoView({
                behavior: 'smooth',
                block: 'center',
                inline: 'nearest'
            });
        }
    }

    function handleInputBlur(e) {
        // When all inputs lose focus, check if keyboard should be hidden
        setTimeout(() => {
            const activeElement = document.activeElement;
            const isInputFocused = activeElement &&
                (activeElement.tagName === 'INPUT' || activeElement.tagName === 'TEXTAREA') &&
                activeElement.closest('#create-card-form');

            if (!isInputFocused) {
                // No form input is focused, stop monitoring
                setTimeout(stopKeyboardMonitoring, 500);
            }
        }, 100);
    }

    // Update viewport height on orientation change
    window.addEventListener('orientationchange', () => {
        setTimeout(() => {
            originalViewportHeight = window.innerHeight;
            checkKeyboardVisibility();

            // Special handling for landscape mode
            const isLandscape = window.innerWidth > window.innerHeight;
            console.log('Orientation changed, landscape:', isLandscape);

            if (isLandscape) {
                // In landscape mode, ensure adequate scrollable space even before keyboard appears
                const createSection = document.getElementById('create-section');
                if (createSection && !createSection.classList.contains('hidden')) {
                    console.log('Setting up landscape viewport for create section');

                    // Pre-set taller viewport for landscape mode
                    const body = document.body;
                    const main = document.querySelector('main');
                    const extraHeight = window.innerHeight * 1.9; // 1.9x height for better scrolling (reduced from 2.5x)

                    body.style.minHeight = extraHeight + 'px';
                    if (main) main.style.minHeight = extraHeight + 'px';
                    createSection.style.minHeight = extraHeight + 'px';

                    setTimeout(() => {
                        scrollToTopOfSection();
                    }, 300);
                }
            } else {
                // Portrait mode - reset to normal heights
                const body = document.body;
                const main = document.querySelector('main');
                const createSection = document.getElementById('create-section');

                if (!document.body.classList.contains('keyboard-visible')) {
                    body.style.minHeight = '';
                    if (main) main.style.minHeight = '';
                    if (createSection) createSection.style.minHeight = '';
                }
            }
        }, 500);
    });

    // Set up input focus handling whenever the create section becomes visible
    const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
            if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
                const createSection = document.getElementById('create-section');
                if (createSection && !createSection.classList.contains('hidden')) {
                    // Small delay to ensure elements are rendered
                    setTimeout(setupInputFocusHandling, 100);
                } else {
                    // Section is hidden, stop monitoring
                    stopKeyboardMonitoring();
                }
            }
        });
    });

    // Observe changes to section visibility
    const sections = document.querySelectorAll('.section');
    sections.forEach(section => {
        observer.observe(section, {
            attributes: true,
            attributeFilter: ['class']
        });
    });

    // Initial setup if create section is already visible
    setTimeout(() => {
        const createSection = document.getElementById('create-section');
        if (createSection && !createSection.classList.contains('hidden')) {
            setupInputFocusHandling();
        }
    }, 100);

    console.log('Mobile keyboard handling initialized');
}// Settings functions
function showSettingsMenu() {
    const settingsMenu = document.getElementById('settings-menu');
    const aboutSection = document.getElementById('about-section');

    // Show the main settings menu, hide the about section
    settingsMenu.style.display = 'block';
    aboutSection.style.display = 'none';

    // Set up event listeners for settings if not already done
    setupSettingsEventListeners();
}

function setupSettingsEventListeners() {
    // Remove existing listeners to prevent duplicates
    const aboutBtn = document.getElementById('about-btn');
    const backToSettingsBtn = document.getElementById('back-to-settings');

    if (aboutBtn && !aboutBtn.hasAttribute('data-listener-added')) {
        aboutBtn.addEventListener('click', showAboutSection);
        aboutBtn.setAttribute('data-listener-added', 'true');
    }

    if (backToSettingsBtn && !backToSettingsBtn.hasAttribute('data-listener-added')) {
        backToSettingsBtn.addEventListener('click', showSettingsMenu);
        backToSettingsBtn.setAttribute('data-listener-added', 'true');
    }
}

function showAboutSection() {
    const settingsMenu = document.getElementById('settings-menu');
    const aboutSection = document.getElementById('about-section');

    // Hide the settings menu, show the about section
    settingsMenu.style.display = 'none';
    aboutSection.style.display = 'block';
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

function editCurrentReviewCard() {
    if (!currentCard) {
        showError('No card selected');
        return;
    }

    console.log('Editing current review card:', currentCard.id);

    // Set flag to indicate we're editing from review
    editingFromReview = true;

    // Use the existing editCard function
    editCard(currentCard.id);
}

async function deleteCurrentReviewCard() {
    if (!currentCard) {
        showError('No card selected');
        return;
    }

    console.log('Deleting current review card:', currentCard.id);

    try {
        // Store for undo
        lastDeletedCard = currentCard;

        // Delete the card
        await invoke('delete_card', { id: currentCard.id });

        // Remove from current review session
        currentReviewCards.splice(currentCardIndex, 1);

        // Update the review session
        if (currentReviewCards.length === 0) {
            // No more cards to review
            finishReview();
            showSuccessWithUndo('Card deleted. Review session complete.');
        } else {
            // Adjust index if we're at the end
            if (currentCardIndex >= currentReviewCards.length) {
                currentCardIndex = currentReviewCards.length - 1;
            }

            // Show next card or finish if no more cards
            if (currentCardIndex < currentReviewCards.length) {
                showCurrentCard();
            } else {
                finishReview();
            }

            showSuccessWithUndo('Card deleted');
        }

        // Update stats and card list
        await loadReviewStats();
        await loadCards();

    } catch (error) {
        console.error('Failed to delete card during review:', error);
        showError('Failed to delete card: ' + error);
    }
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
        if (editMode && editingCardId) {
            // Update existing card
            const updatedCard = await invoke('update_card', {
                id: editingCardId,
                request: {
                    front: front,
                    back: back,
                    tag: tag
                }
            });

            showSuccess('Card updated successfully!');

            // If we were editing from review, update the current review session
            if (editingFromReview) {
                // Update the card in the current review session
                const cardIndex = currentReviewCards.findIndex(card => card.id === editingCardId);
                if (cardIndex !== -1) {
                    currentReviewCards[cardIndex] = updatedCard;
                }

                // Update the current card if it's the one we just edited
                if (currentCard && currentCard.id === editingCardId) {
                    currentCard = updatedCard;
                    // Refresh the card display with updated content
                    document.getElementById('card-front-text').textContent = updatedCard.front;
                    document.getElementById('card-back-text').textContent = updatedCard.back;
                }

                // Reset edit state
                editMode = false;
                editingCardId = null;
                editingFromReview = false;

                // Update UI
                document.getElementById('create-section-title').textContent = 'Create New Card';
                document.getElementById('create-card-submit').textContent = 'Create Card';
                document.getElementById('cancel-edit-btn').classList.add('hidden');

                // Clear form and return to review
                document.getElementById('create-card-form').reset();
                showSection('review');
            } else {
                // Regular edit from browse - use the existing cancelEdit flow
                cancelEdit();
            }

            // Update cards list and stats
            await loadCards();
            await loadReviewStats();
            return; // Exit early for edit mode
        } else {
            // Create new card
            await invoke('create_card', {
                request: {
                    front: front,
                    back: back,
                    tag: tag
                }
            });
            showSuccess('Card created successfully!');
        }

        // Clear form
        document.getElementById('create-card-form').reset();

        // Scroll to top of the create section - use multiple strategies for reliability
        setTimeout(() => {
            scrollToTopOfSection();
        }, 100);

        setTimeout(() => {
            // Force scroll to absolute top as fallback
            window.scrollTo({
                top: 0,
                behavior: 'smooth'
            });
        }, 300);

        // Update stats and cards
        await loadReviewStats();
        await loadCards();

    } catch (error) {
        console.error('Failed to create/update card:', error);
        showError(editMode ? 'Failed to update card' : 'Failed to create card');
    }
}

async function editCard(cardId) {
    try {
        console.log('Editing card:', cardId);

        // Get card data
        const card = await invoke('get_card', { id: cardId });
        if (!card) {
            showError('Card not found');
            return;
        }

        // Enter edit mode
        editMode = true;
        editingCardId = cardId;

        // Update UI
        document.getElementById('create-section-title').textContent = 'Edit Card';
        document.getElementById('create-card-submit').textContent = 'Update Card';
        document.getElementById('cancel-edit-btn').classList.remove('hidden');

        // Populate form with existing data
        document.getElementById('card-front-input').value = card.front;
        document.getElementById('card-back-input').value = card.back;
        document.getElementById('card-tag-input').value = card.tag || '';

        // Switch to create section
        showSection('create');

        // Focus on the front input
        setTimeout(() => {
            document.getElementById('card-front-input').focus();
        }, 100);

    } catch (error) {
        console.error('Failed to load card for editing:', error);
        showError('Failed to load card for editing');
    }
}

function cancelEdit() {
    // Reset edit mode
    editMode = false;
    editingCardId = null;

    // Update UI
    document.getElementById('create-section-title').textContent = 'Create New Card';
    document.getElementById('create-card-submit').textContent = 'Create Card';
    document.getElementById('cancel-edit-btn').classList.add('hidden');

    // Clear form
    document.getElementById('create-card-form').reset();

    // Return to the appropriate section
    if (editingFromReview) {
        editingFromReview = false;
        showSection('review');
    } else {
        showSection('browse');
    }
}

async function loadCards() {
    console.log('=== LOADING CARDS ===');
    try {
        const cards = await invoke('get_cards');
        console.log('Loaded cards from backend:', cards.length, 'cards');
        console.log('Card IDs loaded:', cards.map(c => c.id));
        allCards = cards; // Cache for filtering
        displayCards(cards);
        console.log('Cards displayed successfully');
    } catch (error) {
        console.error('Error loading cards:', error);
        showError('Failed to load cards');
    }
}

function displayCards(cards) {
    const cardsList = document.getElementById('cards-list');
    const bulkActionsBtn = document.getElementById('bulk-actions-btn');

    console.log('displayCards called with', cards.length, 'cards');
    console.log('Selected cards before display:', Array.from(selectedCards));

    if (cards.length === 0) {
        cardsList.innerHTML = '<p class="text-zinc-400 text-center py-8">No cards found.</p>';
        bulkActionsBtn.classList.add('hidden');
        console.log('No cards to display, hiding bulk actions');
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
                <div class="flex space-x-1">
                    <button data-card-id="${card.id}" class="edit-card-btn text-blue-400 hover:text-blue-300 p-1 rounded hover:bg-blue-400/10 transition-colors" title="Edit card">
                        ✏️
                    </button>
                    <button data-card-id="${card.id}" class="delete-card-btn text-red-400 hover:text-red-300 p-1 rounded hover:bg-red-400/10 transition-colors" title="Delete card">
                        🗑️
                    </button>
                </div>
            </div>
            <div class="text-xs text-zinc-500 mt-2">
                Reviews: ${card.review_count} | Interval: ${card.interval} days
                ${card.next_review ? `| Next: ${new Date(card.next_review).toLocaleDateString()}` : ''}
            </div>
        </div>`;
    }).join('');

    updateSelectionControls();
    console.log('Cards displayed and selection controls updated');
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

    // Clear any existing countdown and timeout
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
    }
    if (notificationTimeouts.success) {
        clearTimeout(notificationTimeouts.success);
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

    notificationTimeouts.success = setTimeout(() => {
        successEl.classList.add('hidden');
        if (notificationCountdowns.success) {
            clearInterval(notificationCountdowns.success);
            notificationCountdowns.success = null;
        }
        notificationTimeouts.success = null;
    }, timeout);
}

function showSuccessWithUndo(message) {
    const successEl = document.getElementById('success-message');
    const timeout = CONFIG.UNDO_TIMEOUT;
    let timeLeft = timeout / 1000; // Convert to seconds with decimals

    // Clear any existing countdown and timeout
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
    }
    if (notificationTimeouts.success) {
        clearTimeout(notificationTimeouts.success);
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
    notificationTimeouts.success = setTimeout(() => {
        successEl.classList.add('hidden');
        if (notificationCountdowns.success) {
            clearInterval(notificationCountdowns.success);
            notificationCountdowns.success = null;
        }
        notificationTimeouts.success = null;
    }, timeout);
}

async function undoDelete() {
    // Check if we have bulk deleted cards or single deleted card
    const hasBulkDeletes = lastDeletedCards.length > 0;
    const hasSingleDelete = lastDeletedCard !== null;

    if (!hasBulkDeletes && !hasSingleDelete) {
        showError('No cards to restore');
        return;
    }

    // Clear the countdown and timeout, then hide the undo message immediately
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
        notificationCountdowns.success = null;
    }
    if (notificationTimeouts.success) {
        clearTimeout(notificationTimeouts.success);
        notificationTimeouts.success = null;
    }
    document.getElementById('success-message').classList.add('hidden');

    try {
        if (hasBulkDeletes) {
            console.log('Undoing bulk delete for cards:', lastDeletedCards.length);

            // Recreate all the deleted cards
            for (const card of lastDeletedCards) {
                await invoke('create_card', {
                    request: {
                        front: card.front,
                        back: card.back,
                        tag: card.tag
                    }
                });
            }

            showSuccess(`Restored ${lastDeletedCards.length} card${lastDeletedCards.length > 1 ? 's' : ''} successfully`);
            console.log('Bulk cards restored successfully');

            // Clear the stored cards
            lastDeletedCards = [];
        } else {
            console.log('Undoing delete for card:', lastDeletedCard.front);

            // Recreate the single card
            await invoke('create_card', {
                request: {
                    front: lastDeletedCard.front,
                    back: lastDeletedCard.back,
                    tag: lastDeletedCard.tag
                }
            });

            showSuccess('Card restored successfully');
            console.log('Card restored successfully');

            // Clear the stored card
            lastDeletedCard = null;
        }

        await loadCards();
        await loadReviewStats();

    } catch (error) {
        console.error('Failed to restore cards:', error);
        showError('Failed to restore cards');
    }
}

function dismissUndo() {
    // Clear the countdown and timeout, then hide the undo message immediately
    if (notificationCountdowns.success) {
        clearInterval(notificationCountdowns.success);
        notificationCountdowns.success = null;
    }
    if (notificationTimeouts.success) {
        clearTimeout(notificationTimeouts.success);
        notificationTimeouts.success = null;
    }
    document.getElementById('success-message').classList.add('hidden');

    // Clear the stored card since user dismissed the undo option
    lastDeletedCard = null;
}

function showError(message) {
    const errorEl = document.getElementById('error-message');
    const timeout = CONFIG.ERROR_TIMEOUT;
    let timeLeft = timeout / 1000; // Convert to seconds with decimals

    // Clear any existing countdown and timeout
    if (notificationCountdowns.error) {
        clearInterval(notificationCountdowns.error);
    }
    if (notificationTimeouts.error) {
        clearTimeout(notificationTimeouts.error);
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

    notificationTimeouts.error = setTimeout(() => {
        errorEl.classList.add('hidden');
        if (notificationCountdowns.error) {
            clearInterval(notificationCountdowns.error);
            notificationCountdowns.error = null;
        }
        notificationTimeouts.error = null;
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
    bulkTagSelect.innerHTML = '<option value="">Select Tag</option>';
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
    const bulkActionsBtn = document.getElementById('bulk-actions-btn');
    const bulkInstructions = document.getElementById('bulk-instructions');
    const isVisible = !selectionControls.classList.contains('hidden');

    console.log('toggleBulkMode called, currently visible:', isVisible);

    if (isVisible) {
        // Hide bulk mode
        selectionControls.classList.add('hidden');
        bulkInstructions.classList.add('hidden');
        selectedCards.clear();
        bulkActionsBtn.textContent = 'Bulk Actions';
        bulkActionsBtn.classList.remove('bg-orange-600', 'hover:bg-orange-700');
        bulkActionsBtn.classList.add('bg-blue-600', 'hover:bg-blue-700');
        displayCards(allCards);
        console.log('Bulk mode disabled');
    } else {
        // Show bulk mode
        selectionControls.classList.remove('hidden');
        bulkInstructions.classList.remove('hidden');
        bulkActionsBtn.textContent = 'Exit Bulk Mode';
        bulkActionsBtn.classList.remove('bg-blue-600', 'hover:bg-blue-700');
        bulkActionsBtn.classList.add('bg-orange-600', 'hover:bg-orange-700');
        updateSelectionControls(); // Initialize the state
        console.log('Bulk mode enabled');
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
    const bulkTagApplyBtn = document.getElementById('bulk-tag-apply');

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
    const hasSelection = selectedCards.size > 0;
    bulkDeleteBtn.disabled = !hasSelection;
    bulkTagApplyBtn.disabled = !hasSelection;

    console.log('Selection controls updated:', {
        selectedCount: selectedCards.size,
        buttonsDisabled: !hasSelection
    });
}

async function bulkDeleteCards() {
    console.log('bulkDeleteCards called, selectedCards:', selectedCards);
    if (selectedCards.size === 0) {
        console.log('No cards selected for deletion');
        showError('Please select cards to delete first');
        return;
    }

    const cardIds = Array.from(selectedCards);
    console.log('Card IDs to delete:', cardIds, 'Types:', cardIds.map(id => typeof id));

    console.log('Proceeding with bulk deletion (no confirmation)...');
    console.log('Attempting to delete cards:', cardIds);
    console.log('Selected cards set before deletion:', selectedCards);

    try {
        // First, get all the cards before deleting them (for undo functionality)
        console.log('Getting card data before deletion...');
        const cardsToDelete = [];
        for (const cardId of cardIds) {
            try {
                const card = await invoke('get_card', { id: cardId });
                if (card) {
                    cardsToDelete.push(card);
                }
            } catch (error) {
                console.warn('Could not retrieve card for undo:', cardId, error);
            }
        }
        console.log('Retrieved cards for potential undo:', cardsToDelete);

        // Store for bulk undo
        lastDeletedCards = cardsToDelete;
        lastDeletedCard = null; // Clear single card undo

        console.log('Calling backend delete_multiple_cards with:', { cardIds: cardIds });
        const result = await invoke('delete_multiple_cards', { cardIds: cardIds });
        console.log('Backend delete result:', result);

        showSuccessWithUndo(`Deleted ${cardIds.length} card${cardIds.length > 1 ? 's' : ''}`);

        // Clear selection first, before refreshing UI
        selectedCards.clear();
        console.log('Cleared selected cards:', Array.from(selectedCards));

        // Force immediate UI update
        console.log('Refreshing UI after bulk delete...');

        // Add a small delay to ensure backend has processed
        await new Promise(resolve => setTimeout(resolve, 100));

        await loadCards();
        await loadReviewStats();
        updateSelectionControls();

        console.log('Bulk delete completed successfully');
    } catch (error) {
        console.error('Failed to delete cards:', error);
        showError(`Failed to delete cards: ${error}`);
    }
}

async function bulkUpdateTag() {
    const bulkTagSelect = document.getElementById('bulk-tag-select');
    const bulkTagInput = document.getElementById('bulk-tag-input');
    const newTag = bulkTagInput.value.trim() || bulkTagSelect.value;

    console.log('bulkUpdateTag called, selectedCards:', selectedCards, 'newTag:', newTag);

    if (selectedCards.size === 0) {
        console.log('No cards selected for tag update');
        showError('Please select cards first');
        return;
    }

    if (!newTag) {
        console.log('No tag specified');
        showError('Please select a tag or enter a new one');
        return;
    }

    const cardIds = Array.from(selectedCards);
    console.log('Card IDs for tag update:', cardIds, 'Types:', cardIds.map(id => typeof id));
    console.log('Attempting to update tag for cards:', cardIds, 'to:', newTag);

    try {
        const request = {
            card_ids: cardIds,
            tag: newTag
        };

        console.log('Calling backend bulk_update_tag with:', request);
        const result = await invoke('bulk_update_tag', { request });
        console.log('Backend tag update result:', result);

        showSuccess(`Updated tag for ${cardIds.length} cards to "${newTag}"`);
        selectedCards.clear();

        // Force immediate UI update with delay
        console.log('Refreshing UI after bulk tag update...');
        await new Promise(resolve => setTimeout(resolve, 100));

        await loadCards();
        await loadTags(); // Refresh tags in case it's a new one
        bulkTagSelect.value = '';
        bulkTagInput.value = '';
        updateSelectionControls();

        console.log('Bulk tag update completed successfully');
    } catch (error) {
        console.error('Failed to update tag:', error);
        showError('Failed to update tag');
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
