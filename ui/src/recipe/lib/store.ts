import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AxiosInstance } from 'axios'
import type {
    MenuState,
    RecipeSummary,
    RecipeMeta,
    ApiResponse,
} from './structs'

function mealName(): string {
    const now = new Date()
    const h = now.getHours()
    const m = now.getMonth() + 1
    const d = now.getDate()
    const mm = String(m).padStart(2, '0')
    const dd = String(d).padStart(2, '0')
    let meal = '午餐'
    if (h >= 6 && h < 10) meal = '早餐'
    else if (h >= 10 && h < 14) meal = '午餐'
    else if (h >= 14 && h < 18) meal = '下午茶'
    else if (h >= 18 && h < 22) meal = '晚餐'
    else meal = '夜宵'
    return `${mm}-${dd} ${meal}`
}

export const useRecipeStore = defineStore('recipe', () => {
    const screen = ref<'menu' | 'cooking'>('menu')

    const recipes = ref<RecipeSummary[]>([])
    const categories = computed(() => [...new Set(recipes.value.map((r) => r.category))].sort())

    // Current menu (shared)
    const menu = ref<MenuState | null>(null)

    // Cooking
    const currentRecipe = ref<RecipeMeta | null>(null)
    const currentStep = ref(0)

    // Polling
    let pollTimer: ReturnType<typeof setInterval> | null = null

    async function poll(api: AxiosInstance) {
        try {
            const res = await api.get<ApiResponse<MenuState>>('current')
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
    }

    function startPolling(api: AxiosInstance) {
        stopPolling()
        poll(api)
        pollTimer = setInterval(() => poll(api), 2000)
    }

    function stopPolling() {
        if (pollTimer) {
            clearInterval(pollTimer)
            pollTimer = null
        }
    }

    // ── Actions ──

    async function init(api: AxiosInstance) {
        await loadRecipes(api)
        await poll(api)
        if (!menu.value || !menu.value.filename) {
            await newMenu(api, mealName(), 2)
        }
        startPolling(api)
    }

    async function loadRecipes(api: AxiosInstance, category?: string) {
        const params = category ? { category } : {}
        const res = await api.get<ApiResponse<RecipeSummary[]>>('recipes', { params })
        if (res.data.ok && res.data.data) {
            recipes.value = res.data.data
        }
    }

    async function loadRecipeDetail(api: AxiosInstance, id: string): Promise<RecipeMeta | null> {
        const res = await api.get<ApiResponse<RecipeMeta>>(`recipes/${id}`)
        if (res.data.ok && res.data.data) {
            return res.data.data
        }
        return null
    }

    async function newMenu(api: AxiosInstance, name: string, diners: number = 2) {
        stopPolling()
        try {
            const res = await api.post<ApiResponse<MenuState>>('menus', {
                name: name || mealName(),
                diners,
            })
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
        startPolling(api)
    }

    async function switchToMenu(api: AxiosInstance, filename: string) {
        try {
            await api.post('current', { filename })
        } catch { /* ignore */ }
    }

    // ── Shared toggle ──
    let toggleQueue: string[] = []
    let toggleTimer: ReturnType<typeof setTimeout> | null = null

    async function toggleRecipe(api: AxiosInstance, recipeId: string) {
        if (!menu.value?.filename) return
        // Optimistic local update
        const idx = menu.value.menu_recipes.findIndex(mr => mr.id === recipeId)
        if (idx >= 0) {
            menu.value.menu_recipes.splice(idx, 1)
        } else {
            // Default portions = recipe's own servings (will be corrected by server response)
            const recipe = recipes.value.find(r => r.id === recipeId)
            const portions = recipe?.servings || 2
            menu.value.menu_recipes.push({ id: recipeId, portions })
        }
        // Debounced server sync
        toggleQueue.push(recipeId)
        if (toggleTimer) clearTimeout(toggleTimer)
        toggleTimer = setTimeout(async () => {
            const ids = [...toggleQueue]
            toggleQueue = []
            for (const id of ids) {
                try {
                    const res = await api.post<ApiResponse<MenuState>>(
                        `menus/${menu.value!.filename}/toggle`,
                        { recipe_id: id }
                    )
                    if (res.data.ok && res.data.data) {
                        menu.value = res.data.data
                    }
                } catch { /* ignore */ }
            }
        }, 200)
    }

    async function adjustPortion(api: AxiosInstance, recipeId: string, delta: number) {
        if (!menu.value?.filename) return
        const mr = menu.value.menu_recipes.find(mr => mr.id === recipeId)
        if (!mr) return
        const newPortion = Math.max(0.5, Math.round((mr.portions + delta) * 2) / 2)
        mr.portions = newPortion
        try {
            const res = await api.post<ApiResponse<MenuState>>(
                `menus/${menu.value.filename}/portion`,
                { recipe_id: recipeId, portions: newPortion }
            )
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
    }

    // ── Custom dishes ──

    async function addCustomDish(api: AxiosInstance, name: string) {
        if (!menu.value?.filename || !name.trim()) return
        try {
            const res = await api.post<ApiResponse<MenuState>>(
                `menus/${menu.value.filename}/custom`,
                { name: name.trim() }
            )
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
    }

    async function removeCustomDish(api: AxiosInstance, index: number) {
        if (!menu.value?.filename) return
        try {
            const res = await api.delete<ApiResponse<MenuState>>(
                `menus/${menu.value.filename}/custom/${index}`
            )
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
    }

    async function adjustCustomPortion(api: AxiosInstance, index: number, portions: number) {
        if (!menu.value?.filename) return
        try {
            const res = await api.post<ApiResponse<MenuState>>(
                `menus/${menu.value.filename}/custom/${index}/portion`,
                { portions }
            )
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
    }

    async function updateDiners(api: AxiosInstance, diners: number) {
        if (!menu.value?.filename) return
        try {
            const res = await api.put<ApiResponse<MenuState>>(
                `menus/${menu.value.filename}`,
                { diners }
            )
            if (res.data.ok && res.data.data) {
                menu.value = res.data.data
            }
        } catch { /* ignore */ }
    }

    function getCoverUrl(id: string, cover?: string): string {
        if (cover) return `api/image/${id}`
        return ''
    }

    return {
        screen,
        recipes,
        categories,
        menu,
        currentRecipe,
        currentStep,
        init,
        startPolling,
        stopPolling,
        loadRecipes,
        loadRecipeDetail,
        newMenu,
        switchToMenu,
        toggleRecipe,
        adjustPortion,
        addCustomDish,
        removeCustomDish,
        adjustCustomPortion,
        updateDiners,
        getCoverUrl,
        mealName,
    }
})
