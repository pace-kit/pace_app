// Explicit, bounded requests. Credentials live only in component/request memory.
export async function projectRequest(method, url, token, etag, body, limit) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 30000);
    try {
        const headers = { Authorization: `Bearer ${token}` };
        if (body !== undefined) headers['Content-Type'] = 'application/json';
        if (etag !== undefined) headers['If-Match'] = etag;
        const response = await fetch(url, {
            method, headers, body, signal: controller.signal,
            credentials: 'omit', redirect: 'error', cache: 'no-store',
            referrerPolicy: 'no-referrer', mode: 'cors',
        });
        if (Number(response.headers.get('Content-Length')) > limit) throw new Error('too_large');
        const reader = response.body?.getReader();
        const decoder = new TextDecoder('utf-8', { fatal: true });
        const parts = [];
        let size = 0;
        if (reader) {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                size += value.byteLength;
                if (size > limit) throw new Error('too_large');
                parts.push(decoder.decode(value, { stream: true }));
            }
            parts.push(decoder.decode());
        }
        return { status: response.status, etag: response.headers.get('ETag'), text: parts.join('') };
    } finally {
        clearTimeout(timeout);
        controller.abort();
    }
}
