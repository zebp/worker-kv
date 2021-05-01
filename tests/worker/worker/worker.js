addEventListener("fetch", (event) => {
    event.respondWith(handleRequest(event.request));
});

/**
 * Fetch and log a request
 * @param {Request} request
 */
async function handleRequest(request) {
    const { runStep } = wasm_bindgen;
    await wasm_bindgen(wasm);

    try {
        const url = new URL(request.url);
        const stepStr = url.pathname.substring(1);
        const step = parseInt(stepStr);

        await runStep(step);

        return new Response('{"success":true}', {
            status: 200,
            headers: {
                "Content-type": "application/json",
            },
        });
    } catch (error) {
        return new Response(JSON.stringify(error), {
            status: 500,
            headers: {
                "Content-type": "application/json",
            },
        });
    }
}
