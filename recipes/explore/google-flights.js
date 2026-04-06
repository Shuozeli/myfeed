// Exploration script for Google Flights page structure
// Run with: CDP_ENDPOINT=http://browser-yuacx:9222 cargo run --bin explore_google_flights

use pwright_bridge::{Browser, BrowserConfig};
use serde_json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cdp_url = env::var("CDP_ENDPOINT").unwrap_or_else(|_| "http://browser-yuacx:9222".to_string());

    let browser = Browser::connect(BrowserConfig {
        cdp_url,
        ..Default::default()
    }).await?;

    let tab = browser.new_tab("about:blank").await?;
    let page = tab.page();

    // Navigate to Google Flights
    let url = "https://www.google.com/travel/flights";
    println!("Navigating to: {}", url);
    page.goto(url).await?;
    page.wait_for_timeout(3000).await;

    // Run setup script first
    let setup_script = r#"
        (async () => {
            await new Promise(r => setTimeout(r, 3000));

            const originSelectors = [
                'input[aria-label*="origin"]',
                'input[placeholder*="From"]',
                '[aria-label*="Where from"]',
                'input[type="text"]'
            ];

            let originInput = null;
            for (const sel of originSelectors) {
                const el = document.querySelector(sel);
                if (el && el.offsetParent !== null) {
                    originInput = el;
                    break;
                }
            }

            if (originInput) {
                originInput.click();
                await new Promise(r => setTimeout(r, 500));

                const originCity = 'San Francisco';
                const destCity = 'Beijing';

                // Type origin city
                for (const char of originCity) {
                    originInput.value += char;
                    originInput.dispatchEvent(new Event('input', { bubbles: true }));
                    await new Promise(r => setTimeout(r, 50));
                }

                await new Promise(r => setTimeout(r, 1000));

                // Select first autocomplete result
                const dropdown = document.querySelector('[role="listbox"], [role="presentation"]');
                if (dropdown) {
                    const firstOption = dropdown.querySelector('li, [role="option"]');
                    if (firstOption) {
                        firstOption.click();
                        await new Promise(r => setTimeout(r, 500));
                    }
                }

                // Find and fill destination
                const destSelectors = [
                    'input[aria-label*="destination"]',
                    'input[placeholder*="To"]',
                    '[aria-label*="Where to"]'
                ];

                let destInput = null;
                for (const sel of destSelectors) {
                    const el = document.querySelector(sel);
                    if (el && el.offsetParent !== null) {
                        destInput = el;
                        break;
                    }
                }

                if (destInput) {
                    destInput.click();
                    await new Promise(r => setTimeout(r, 500));

                    for (const char of destCity) {
                        destInput.value += char;
                        destInput.dispatchEvent(new Event('input', { bubbles: true }));
                        await new Promise(r => setTimeout(r, 50));
                    }

                    await new Promise(r => setTimeout(r, 1000));

                    const dropdown2 = document.querySelector('[role="listbox"], [role="presentation"]');
                    if (dropdown2) {
                        const firstOption2 = dropdown2.querySelector('li, [role="option"]');
                        if (firstOption2) {
                            firstOption2.click();
                            await new Promise(r => setTimeout(r, 500));
                        }
                    }
                }

                return { status: 'search_filled' };
            }
            return { status: 'no_input_found' };
        })()
    "#;

    page.evaluate(setup_script).await?;
    page.wait_for_timeout(5000).await;

    // Now extract structured data
    let extract_script = r#"
        (async () => {
            const bodyText = document.body.innerText;

            // Get all prices in document order
            const priceRegex = /\$[\d,]+(?:\.\d{2})?/g;
            const prices = [];
            let priceMatch;
            while ((priceMatch = priceRegex.exec(bodyText)) !== null) {
                const contextStart = Math.max(0, priceMatch.index - 200);
                const contextEnd = Math.min(bodyText.length, priceMatch.index + 200);
                const context = bodyText.substring(contextStart, contextEnd);
                prices.push({
                    value: priceMatch[0],
                    pos: priceMatch.index,
                    context: context.replace(/\n/g, ' ').substring(0, 200)
                });
            }

            // Get all flight-related blocks
            // Look for common flight card selectors
            const flightCards = document.querySelectorAll('[data-merchant*="flight"], [class*="flight"], .uEa");
            const cardData = [];
            flightCards.forEach((card, i) => {
                cardData.push({
                    index: i,
                    text: card.innerText.substring(0, 300),
                    tag: card.tagName,
                    class: card.className.substring(0, 100)
                });
            });

            // Get aria labels with price info
            const priceElements = document.querySelectorAll('[aria-label*="$"], [aria-label*="USD"]');
            const ariaPrices = [];
            priceElements.forEach(el => {
                const label = el.getAttribute('aria-label');
                if (label) ariaPrices.push(label);
            });

            // Look for the main results section
            const mainSection = document.querySelector('#app, main, [role="main"]');
            const mainText = mainSection ? mainSection.innerText.substring(0, 3000) : '';

            return JSON.stringify({
                totalPrices: prices.length,
                prices: prices.slice(0, 30),
                cardCount: flightCards.length,
                cardData: cardData.slice(0, 5),
                ariaPrices: ariaPrices.slice(0, 20),
                mainTextSnippet: mainText.substring(0, 2000)
            });
        })()
    "#;

    let data: serde_json::Value = page.evaluate(extract_script).await?;
    println!("\n=== Page Exploration Data ===");
    println!("{}", serde_json::to_string_pretty(&data).unwrap());

    // Also get body text for full context
    let body_text: String = page.evaluate("document.body.innerText").await?;
    println!("\n=== Body Text (first 5000 chars) ===");
    println!("{}", &body_text[..body_text.len().min(5000)]);

    tab.close().await?;
    Ok(())
}