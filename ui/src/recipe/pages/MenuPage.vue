<script setup lang="ts">
import { ref, computed, getCurrentInstance } from 'vue'
import type { AxiosInstance } from 'axios'
import { useRecipeStore } from '../lib/store'
import RecipeCard from '../components/RecipeCard.vue'
import IngredientList from '../components/IngredientList.vue'
import type { ApiResponse, MenuSummary, RecipeMeta, RecipeSummary } from '../lib/structs'

const api = getCurrentInstance()?.appContext.config.globalProperties.$api as AxiosInstance
const store = useRecipeStore()

const activeCategory = ref<string>('')
const showSummary = ref(false)
const showMenus = ref(false)
const menus = ref<MenuSummary[]>([])

// Selected dishes (resolved from menu_recipes)
const selectedDishes = computed(() => {
    if (!store.menu) return []
    const recipeMap = new Map(store.recipes.map(r => [r.id, r]))
    return store.menu.menu_recipes
        .map(mr => {
            const r = recipeMap.get(mr.id)
            return r ? { ...r, portions: mr.portions, menuId: mr.id } : null
        })
        .filter(Boolean) as (RecipeSummary & { portions: number; menuId: string })[]
})

// ── New menu dialog ──
const showNewDialog = ref(false)
const newDate = ref('')
const newMeal = ref('午餐')
const newDiners = ref(2)
const MEALS = ['早餐', '午餐', '下午茶', '晚餐', '夜宵']

// ── Recipe detail modal ──
const showRecipeDetail = ref(false)
const detailRecipe = ref<RecipeMeta | null>(null)

// ── Diners edit ──
const editingDiners = ref(false)
const dinersInput = ref(2)

function todayStr(): string {
    const now = new Date()
    const y = now.getFullYear()
    const m = String(now.getMonth() + 1).padStart(2, '0')
    const d = String(now.getDate()).padStart(2, '0')
    return `${y}-${m}-${d}`
}

function defaultMeal(): string {
    const h = new Date().getHours()
    if (h >= 6 && h < 10) return '早餐'
    if (h >= 10 && h < 14) return '午餐'
    if (h >= 14 && h < 18) return '下午茶'
    if (h >= 18 && h < 22) return '晚餐'
    return '夜宵'
}

function dateDisplay(dateStr: string): string {
    if (!dateStr) return ''
    const parts = dateStr.split('-')
    if (parts.length !== 3) return dateStr
    return `${parts[1]}-${parts[2]}`
}

function openNewMenuDialog() {
    newDate.value = todayStr()
    newMeal.value = defaultMeal()
    newDiners.value = 2
    showNewDialog.value = true
}

async function createNewMenu() {
    const name = `${dateDisplay(newDate.value)} ${newMeal.value}`
    showNewDialog.value = false
    showMenus.value = false
    await store.loadRecipes(api)
    await store.newMenu(api, name, newDiners.value)
}

const filteredRecipes = computed(() => {
    if (!activeCategory.value) return store.recipes
    return store.recipes.filter((r) => r.category === activeCategory.value)
})

function startEditDiners() {
    dinersInput.value = store.menu?.diners || 2
    editingDiners.value = true
}

function confirmDiners() {
    const n = dinersInput.value
    if (n >= 1 && n <= 9) store.updateDiners(api, n)
    editingDiners.value = false
}

function isSelected(id: string): boolean {
    return store.menu?.menu_recipes.some(mr => mr.id === id) || false
}

function onAdjust(id: string, delta: number) {
    store.adjustPortion(api, id, delta)
}

function onToggle(id: string) {
    store.toggleRecipe(api, id)
}

async function openRecipeDetail(id: string) {
    detailRecipe.value = await store.loadRecipeDetail(api, id)
    if (detailRecipe.value) showRecipeDetail.value = true
}

const selectedCount = computed(() => store.menu?.menu_recipes.length || 0)

// ── Saved menus ──
async function loadMenus() {
    try {
        const res = await api.get<ApiResponse<MenuSummary[]>>('menus')
        if (res.data.ok && res.data.data) {
            menus.value = res.data.data
        }
    } catch { /* ignore */ }
}

async function openSavedMenu(m: MenuSummary) {
    showMenus.value = false
    await store.loadRecipes(api)
    await store.switchToMenu(api, m.filename)
}

async function deleteMenu(m: MenuSummary) {
    if (!confirm(`删除「${m.name}」？`)) return
    try {
        await api.delete(`menus/${m.filename}`)
        await loadMenus()
    } catch { /* ignore */ }
}

function goCooking() {
    store.stopPolling()
    store.screen = 'cooking'
}
</script>

<template>
    <div class="min-h-dvh bg-base-100 text-base-content">
        <!-- Top bar -->
        <div class="navbar bg-base-200 shadow-sm px-2 gap-1 max-md:flex-wrap">
            <button class="btn btn-sm btn-ghost" @click="showMenus = true; loadMenus()">
                📋 菜单
            </button>
            <h1 class="text-base font-bold truncate max-w-32">{{ store.menu?.name || '加载中...' }}</h1>
            <!-- Diners -->
            <div v-if="!editingDiners" class="badge badge-sm cursor-pointer" @click="startEditDiners">
                {{ store.menu?.diners || '?' }}人
            </div>
            <input
                v-else
                v-model.number="dinersInput"
                type="number"
                min="1" max="9"
                class="input input-sm input-bordered w-14"
                @keyup.enter="confirmDiners"
                @blur="confirmDiners"
            />
            <div class="flex-1"></div>
            <button
                class="btn btn-sm"
                :class="selectedCount > 0 ? 'btn-accent' : 'btn-ghost'"
                @click="showSummary = !showSummary"
            >
                {{ selectedCount }}道菜
            </button>
        </div>

        <!-- Category tabs -->
        <div class="flex gap-1 px-3 py-2 overflow-x-auto">
            <button
                class="btn btn-sm"
                :class="activeCategory === '' ? 'btn-primary' : 'btn-ghost'"
                @click="activeCategory = ''"
            >
                全部
            </button>
            <button
                v-for="cat in store.categories"
                :key="cat"
                class="btn btn-sm whitespace-nowrap"
                :class="activeCategory === cat ? 'btn-primary' : 'btn-ghost'"
                @click="activeCategory = cat"
            >
                {{ cat }}
            </button>
        </div>

        <!-- Recipe grid -->
        <div class="px-2 pb-24">
            <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2">
                <RecipeCard
                    v-for="r in filteredRecipes"
                    :key="r.id"
                    :recipe="r"
                    :selected="isSelected(r.id)"
                    :others-selected="false"
                    @toggle="onToggle(r.id)"
                    @detail="openRecipeDetail(r.id)"
                />
            </div>
        </div>

        <!-- Saved menus drawer -->
        <div v-if="showMenus" class="fixed inset-0 z-50 flex" @click.self="showMenus = false">
            <div class="w-72 max-w-[80vw] bg-base-200 h-full overflow-y-auto shadow-xl p-4">
                <div class="flex items-center justify-between mb-3">
                    <h2 class="text-lg font-bold">已保存菜单</h2>
                    <button class="btn btn-sm btn-ghost" @click="showMenus = false">✕</button>
                </div>
                <button class="btn btn-sm btn-primary w-full mb-3" @click="openNewMenuDialog">
                    + 新建菜单
                </button>
                <div class="space-y-2">
                    <div
                        v-for="m in menus"
                        :key="m.filename"
                        class="card bg-base-100 cursor-pointer hover:bg-base-300 transition-colors"
                        :class="{ 'ring-1 ring-accent': m.filename === store.menu?.filename }"
                    >
                        <div class="card-body p-3">
                            <div class="flex items-center justify-between">
                                <div @click="openSavedMenu(m)" class="flex-1 cursor-pointer">
                                    <h3 class="text-base font-semibold">{{ m.name }}</h3>
                                    <p class="text-base text-base-content/50">
                                        {{ m.date }} · {{ m.diners }}人 · {{ m.dish_count }}道
                                    </p>
                                </div>
                                <button class="btn btn-sm btn-ghost btn-error" @click="deleteMenu(m)">🗑</button>
                            </div>
                        </div>
                    </div>
                    <div v-if="menus.length === 0" class="text-base text-base-content/40 text-center py-4">
                        还没有菜单
                    </div>
                </div>
            </div>
            <div class="flex-1 bg-black/50"></div>
        </div>

        <!-- Summary overlay: selected dishes → ingredients → cooking -->
        <div v-if="showSummary" class="fixed inset-0 z-50 bg-base-100 overflow-y-auto">
            <div class="max-w-lg mx-auto p-4 pb-24">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-lg font-bold">{{ selectedCount }}道菜</h2>
                    <button class="btn btn-sm btn-ghost" @click="showSummary = false">✕</button>
                </div>

                <!-- Selected dishes -->
                <div v-if="selectedDishes.length === 0" class="text-center text-base-content/40 py-8">
                    还没有选菜
                </div>
                <div class="space-y-2 mb-6">
                    <div v-for="d in selectedDishes" :key="d.id"
                        class="flex items-center gap-2 p-2 rounded bg-base-200">
                        <img v-if="d.cover_image" :src="`api/image/${d.id}`"
                            class="w-10 h-10 rounded object-cover cursor-pointer"
                            @click="showSummary = false; openRecipeDetail(d.id)" />
                        <div v-else class="w-10 h-10 rounded bg-base-300 flex items-center justify-center text-lg cursor-pointer"
                            @click="showSummary = false; openRecipeDetail(d.id)">🍽️</div>
                        <div class="flex-1 min-w-0 cursor-pointer" @click="showSummary = false; openRecipeDetail(d.id)">
                            <div class="text-sm font-semibold truncate">{{ d.name }}</div>
                            <div class="flex items-center gap-2">
                                <span class="text-sm text-base-content/50">{{ d.servings }}人份</span>
                                <!-- Portion indicator for adjustable recipes -->
                                <span v-if="d.adjustable" class="badge badge-xs">{{ d.portions }}份</span>
                            </div>
                        </div>
                        <!-- +/- buttons for adjustable recipes -->
                        <div v-if="d.adjustable" class="flex items-center gap-0.5">
                            <button class="btn btn-xs btn-ghost w-6 h-6" @click.stop="onAdjust(d.menuId, -0.5)">−</button>
                            <span class="text-xs w-6 text-center">{{ d.portions }}</span>
                            <button class="btn btn-xs btn-ghost w-6 h-6" @click.stop="onAdjust(d.menuId, 0.5)">+</button>
                        </div>
                        <button class="btn btn-sm btn-ghost btn-error" @click.stop="onToggle(d.id)">✕</button>
                    </div>
                </div>

                <div class="divider"></div>

                <!-- Ingredients -->
                <h2 class="text-lg font-bold mb-3">食材清单</h2>
                <IngredientList :ingredients="store.menu?.ingredients || []" />

                <!-- Cooking button -->
                <div class="mt-6">
                    <button class="btn btn-accent btn-lg w-full" @click="showSummary = false; goCooking()"
                        :disabled="selectedCount === 0">
                        🍳 开始做菜
                    </button>
                </div>
            </div>
        </div>

        <!-- New menu dialog -->
        <div v-if="showNewDialog" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" @click.self="showNewDialog = false">
            <div class="card bg-base-200 shadow-xl w-80 max-w-[90vw] p-4">
                <h2 class="text-lg font-bold mb-3">新建菜单</h2>
                <div class="flex flex-col gap-3">
                    <label class="form-control">
                        <span class="label-text">日期</span>
                        <input v-model="newDate" type="date" class="input input-bordered input-sm" />
                    </label>
                    <label class="form-control">
                        <span class="label-text">餐次</span>
                        <div class="flex gap-1 flex-wrap mt-1">
                            <button v-for="m in MEALS" :key="m" class="btn btn-sm"
                                :class="newMeal === m ? 'btn-primary' : 'btn-ghost'" @click="newMeal = m">{{ m }}</button>
                        </div>
                    </label>
                    <label class="form-control">
                        <span class="label-text">用餐人数</span>
                        <div class="flex gap-1 mt-1">
                            <button v-for="n in 9" :key="n" class="btn btn-sm w-8 h-8"
                                :class="newDiners === n ? 'btn-primary' : 'btn-ghost'" @click="newDiners = n">{{ n }}</button>
                        </div>
                    </label>
                    <div class="text-base font-semibold text-center py-1">
                        菜单名：<span class="text-accent">{{ dateDisplay(newDate) }} {{ newMeal }}</span>
                    </div>
                    <div class="flex gap-2">
                        <button class="btn btn-sm btn-primary flex-1" @click="createNewMenu">创建</button>
                        <button class="btn btn-sm btn-ghost" @click="showNewDialog = false">取消</button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Recipe detail modal -->
        <div v-if="showRecipeDetail && detailRecipe" class="fixed inset-0 z-50 bg-base-100 overflow-y-auto">
            <div class="max-w-lg mx-auto">
                <div class="relative">
                    <img v-if="detailRecipe.cover_image" :src="`api/image/${detailRecipe.id}`" :alt="detailRecipe.name"
                        class="w-full h-56 object-cover" />
                    <div v-else class="w-full h-40 bg-base-300 flex items-center justify-center text-6xl">🍽️</div>
                    <button class="absolute top-3 left-3 btn btn-sm btn-circle btn-ghost bg-base-100/70"
                        @click="showRecipeDetail = false">✕</button>
                </div>
                <div class="px-4 pb-24 pt-3">
                    <h1 class="text-xl font-bold">{{ detailRecipe.name }}</h1>
                    <div class="flex gap-3 text-base text-base-content/60 mt-1 mb-4">
                        <span>⏱ {{ detailRecipe.cook_time }}</span>
                        <span>👥 {{ detailRecipe.servings }}人份</span>
                        <span>📊 {{ detailRecipe.difficulty }}</span>
                    </div>
                    <button class="btn btn-sm w-full mb-4"
                        :class="isSelected(detailRecipe.id) ? 'btn-error btn-outline' : 'btn-primary'"
                        @click="onToggle(detailRecipe.id)">
                        {{ isSelected(detailRecipe.id) ? '从菜单中移除' : '➕ 加入菜单' }}
                    </button>
                    <h2 class="text-base font-semibold mb-2">食材</h2>
                    <div class="text-base space-y-1 mb-4">
                        <div v-for="ing in detailRecipe.ingredients" :key="ing.name"
                            class="flex justify-between py-1 border-b border-base-200">
                            <span>{{ ing.name }}</span>
                            <span class="text-base-content/60">{{ ing.amount ? `${ing.amount}${ing.unit || ''}` : ing.hint || '' }}</span>
                        </div>
                    </div>
                    <div class="divider my-2"></div>
                    <div class="prose prose-sm max-w-none" v-html="detailRecipe.body
                        .replace(/^### (.+)$/gm, '<h3 class=\'text-base font-bold mt-4 mb-2\'>$1</h3>')
                        .replace(/^## (.+)$/gm, '<h2 class=\'text-lg font-bold mt-4 mb-2\'>$1</h2>')
                        .replace(/^# (.+)$/gm, '<h1 class=\'text-xl font-bold mt-4 mb-2\'>$1</h1>')
                        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
                        .replace(/^> (.+)$/gm, '<blockquote class=\'border-l-2 border-accent pl-3 my-2 text-base-content/60\'>$1</blockquote>')
                        .replace(/^- (.+)$/gm, '<li class=\'ml-4\'>$1</li>')
                        .replace(/^(\d+)\. (.+)$/gm, '<div class=\'flex gap-2 my-1\'><span class=\'font-bold text-accent\'>$1.</span><span>$2</span></div>')
                        .replace(/\n\n/g, '<br/>')
                    "></div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
