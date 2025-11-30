import { render, screen } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import { LoginPage } from './login-page';

describe('LoginPage', () => {
    it('renders correctly', () => {
        render(
            <BrowserRouter>
                <LoginPage />
            </BrowserRouter>
        );
        expect(screen.getByRole('heading', { name: /Welcome Back/i })).toBeInTheDocument();
    });
});
