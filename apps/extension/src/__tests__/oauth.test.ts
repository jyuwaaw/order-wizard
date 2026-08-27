import { afterEach, describe, expect, it, vi } from 'vitest';
import { createAuthorizationUrl, revokeRefreshToken } from '../config/oauth';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('Cognito authorization URL', () => {
  it('binds the access token to the API and requests exact extension scopes', () => {
    const url = createAuthorizationUrl({
      authorizationEndpoint: 'https://auth.example.com/oauth2/authorize',
      clientId: 'extension-client',
      redirectUri: 'https://extension.example/callback',
      resourceUri: 'https://api.orderwizard.example',
      codeChallenge: 'challenge',
      state: 'state-123',
    });

    expect(url.searchParams.get('resource')).toBe('https://api.orderwizard.example');
    expect(url.searchParams.get('state')).toBe('state-123');
    expect(url.searchParams.get('scope')).toBe(
      'openid email https://api.orderwizard.example/orders.read ' +
        'https://api.orderwizard.example/orders.sync ' +
        'https://api.orderwizard.example/orders.status.write ' +
        'https://api.orderwizard.example/orders.note.write',
    );
  });

  it('revokes the refresh token as a public client', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    await revokeRefreshToken(
      'refresh-token',
      {
        issuer: 'https://issuer.example',
        revocation_endpoint: 'https://auth.example.com/oauth2/revoke',
      },
      { client_id: 'extension-client', token_endpoint_auth_method: 'none' },
    );

    const [, request] = fetchMock.mock.calls[0];
    const body = new URLSearchParams(request.body);
    expect(body.get('token')).toBe('refresh-token');
    expect(body.get('client_id')).toBe('extension-client');
  });
});
