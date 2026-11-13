import journeyHtml from '../public/waste-collection.html?raw';
import landingHtml from '../index.html?raw';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';
import App from './App';

function loadJourney() {
 const html = journeyHtml;
 document.body.innerHTML = html;
 return html;
}

test('offers the journey from the guide without removing finder and map', () => {
 render(<App/>);
 expect(screen.getByRole('link', {name: 'Learn waste collection'})).toHaveAttribute('href', '/waste-collection.html');
 expect(screen.getByRole('button', {name: 'Find a service'})).toBeVisible();
 expect(screen.getByRole('button', {name: 'Explore map'})).toBeVisible();
});

test('ships six named sections, accessible anchors and complete no-JavaScript content', () => {
 const html = loadJourney();
 expect(document.querySelectorAll('main > section')).toHaveLength(6);
 expect(document.querySelector('script')).toBeNull();
 expect(screen.getByRole('heading', {level: 1, name: 'Where does it go, Manassas?'})).toBeVisible();
 expect(screen.getByRole('link', {name: 'Skip to the journey'})).toHaveAttribute('href', '#journey');
 for (const anchor of document.querySelectorAll<HTMLAnchorElement>('a[href^="#"]')) {
  expect(document.getElementById(anchor.hash.slice(1))).not.toBeNull();
 }
 expect(html).toContain('2026-11-10');
 expect(screen.getByText(/A Manassas mailing address or ZIP code does not confirm/)).toBeVisible();
 expect(screen.getByRole('link', {name: 'Back to service finder'})).toHaveAttribute('href', '/');
});

test('teaches glass sorting with independently usable native choices and cited explanations', async () => {
 loadJourney();
 const exercise = screen.getByRole('group', {name: 'Try it: an empty glass jar'});
 const choices = within(exercise).getAllByText(/^(Curbside recycling cart|Purple glass container|Yard waste bag)$/);
 expect(choices).toHaveLength(3);
 const user = userEvent.setup();
 for (const choice of choices) {
  const details = choice.closest('details')!;
  expect(details).not.toHaveAttribute('open');
  await user.click(choice);
  expect(details).toHaveAttribute('open');
  expect(within(details).getByRole('link', {name: 'Read the City recycling guidance'})).toHaveAttribute('href', 'https://www.manassasva.gov/public_works/trash___recycling/recycling.php');
  await user.click(choice);
  expect(details).not.toHaveAttribute('open');
 }
 expect(within(exercise).getByText(/Correct. Clean glass bottles and jars/)).toBeInTheDocument();
});

test('retains current bulk rules and avoids false operational promises', () => {
 loadJourney();
 const bulk = document.getElementById('bulk')!;
 expect(bulk).toHaveTextContent('at least 48 hours before your bulk collection day');
 expect(bulk).toHaveTextContent('up to 3 items');
 expect(bulk).toHaveTextContent('may differ from your regular trash day');
 expect(document.body).toHaveTextContent('does not submit a request to the City');
 expect(document.body).toHaveTextContent('not a live schedule or an eligibility determination');
 expect(document.body).toHaveTextContent('The City pages give different recovery timing');
 for (const link of document.querySelectorAll<HTMLAnchorElement>('a[href^="https:"]')) {
  expect(new URL(link.href).hostname).toBe('www.manassasva.gov');
  expect(link.textContent?.trim()).not.toBe('Official next action');
 }
});


test('keeps the journey reachable from the app when JavaScript is disabled', () => {
 const parsed = new DOMParser().parseFromString(landingHtml, 'text/html');
 expect(parsed.querySelector('noscript a')?.getAttribute('href')).toBe('/waste-collection.html');
});
