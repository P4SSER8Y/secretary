/** @type {import('tailwindcss').Config} */
export default {
    content: ['./src/**/*.{js,ts,vue,html}', './{inbox}/index.html'],
    theme: {
        extend: {},
    },
    plugins: [require('daisyui')],
    darkMode: 'class',
    daisyui: {
        themes: [
            {
                p5: {
                    "primary": "#ff0022",
                    "base-100": "#363636",
                }
            }
        ]
    }
};
