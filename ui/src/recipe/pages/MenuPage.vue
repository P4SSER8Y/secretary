<script setup lang="ts">
import { ref, computed, watch, getCurrentInstance } from 'vue'
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

// Custom (freeform) dishes
const newCustomName = ref('')
const addingCustom = ref(false)

async function addCustom() {
    const name = newCustomName.value.trim()
    if (!name) return
    addingCustom.value = true
    await store.addCustomDish(api, name)
    newCustomName.value = ''
    addingCustom.value = false
}

function removeCustom(index: number) {
    store.removeCustomDish(api, index)
}

function adjustCustom(index: number, delta: number) {
    const cd = store.menu?.custom_dishes[index]
    if (!cd) return
    const newPortion = Math.max(0.5, Math.round((cd.portions + delta) * 2) / 2)
    store.adjustCustomPortion(api, index, newPortion)
}

// ── New menu dialog ──
const showNewDialog = ref(false)
const newDate = ref('')
const newMeal = ref('午餐')
const newDiners = ref(2)
const MEALS = ['早餐', '午餐', '下午茶', '晚餐', '夜宵']

// ── Recipe detail modal ──
const showRecipeDetail = ref(false)
const detailRecipe = ref<RecipeMeta | null>(null)
const detailPortions = ref(0)

function scaleIngredient(ing: import('../lib/structs').Ingredient, ratio: number): import('../lib/structs').Ingredient {
    return {
        ...ing,
        amount: ing.amount != null ? Math.round(ing.amount * ratio * 100) / 100 : undefined,
        sub_ingredients: ing.sub_ingredients?.map(sub => scaleIngredient(sub, ratio)),
    }
}

const detailScaledIngredients = computed(() => {
    if (!detailRecipe.value) return []
    const ratio = detailPortions.value / detailRecipe.value.servings
    if (ratio === 1) return detailRecipe.value.ingredients
    return detailRecipe.value.ingredients.map(ing => scaleIngredient(ing, ratio))
})

function detailAdjust(delta: number) {
    const val = Math.max(0.5, Math.round((detailPortions.value + delta) * 2) / 2)
    if (val === detailPortions.value) return
    detailPortions.value = val
    if (detailRecipe.value && isSelected(detailRecipe.value.id)) {
        store.adjustPortion(api, detailRecipe.value.id, delta)
    }
}

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

function dayOfWeek(dateStr: string): string {
    if (!dateStr) return ''
    const d = new Date(dateStr + 'T00:00:00')
    const days = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
    return days[d.getDay()]
}

function changeDate(delta: number) {
    const d = new Date(newDate.value + 'T00:00:00')
    d.setDate(d.getDate() + delta)
    const y = d.getFullYear()
    const m = String(d.getMonth() + 1).padStart(2, '0')
    const dd = String(d.getDate()).padStart(2, '0')
    newDate.value = `${y}-${m}-${dd}`
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

const CATEGORY_ORDER = ['肉', '菜', '汤']

const orderedCategories = computed(() => {
    const existing = new Set(store.categories)
    const cats = CATEGORY_ORDER.filter(c => existing.has(c))
    const otherCount = store.categories.filter(c => !CATEGORY_ORDER.includes(c)).length
    if (otherCount > 0) cats.push('其它')
    return cats
})

const slideDirection = ref<'left' | 'right'>('right')

function categoryIndex(cat: string): number {
    if (cat === '') return orderedCategories.value.length // "全部" is last
    return orderedCategories.value.indexOf(cat)
}

function switchCategory(cat: string) {
    const oldIdx = categoryIndex(activeCategory.value)
    const newIdx = categoryIndex(cat)
    slideDirection.value = newIdx > oldIdx ? 'left' : 'right'
    activeCategory.value = cat
}

const filteredRecipes = computed(() => {
    if (!activeCategory.value) return store.recipes
    if (activeCategory.value === '其它') return store.recipes.filter((r) => !CATEGORY_ORDER.includes(r.category))
    return store.recipes.filter((r) => r.category === activeCategory.value)
})

function startEditDiners() {
    dinersInput.value = store.menu?.diners || 2
    editingDiners.value = true
}

function confirmDiners() {
    const n = dinersInput.value
    if (n >= 1 && n <= 8) store.updateDiners(api, n)
    editingDiners.value = false
}

const viewMode = ref<'grid' | 'list'>((localStorage.getItem('recipe-view-mode') as 'grid' | 'list') || 'grid')
watch(viewMode, (v) => localStorage.setItem('recipe-view-mode', v))

const categoryCounts = computed(() => {
    const counts: Record<string, number> = {}
    store.recipes.forEach(r => {
        const cat = CATEGORY_ORDER.includes(r.category) ? r.category : '其它'
        counts[cat] = (counts[cat] || 0) + 1
    })
    return counts
})

const syncing = ref(false)
async function syncRecipes() {
    if (syncing.value) return
    syncing.value = true
    try {
        const res = await api.post<ApiResponse<string>>('recipes/sync')
        if (res.data.ok) {
            await store.loadRecipes(api)
        }
    } catch { /* ignore */ }
    finally { syncing.value = false }
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
    if (detailRecipe.value) {
        const existing = store.menu?.menu_recipes.find(mr => mr.id === id)
        detailPortions.value = existing ? existing.portions : detailRecipe.value.servings
        showRecipeDetail.value = true
    }
}

async function addWithPortions() {
    if (!detailRecipe.value || !store.menu?.filename) return
    const id = detailRecipe.value.id
    if (isSelected(id)) {
        onToggle(id)
        showRecipeDetail.value = false
        return
    }
    const desired = detailPortions.value
    onToggle(id)
    await new Promise(r => setTimeout(r, 300))
    const mr = store.menu?.menu_recipes.find(mr => mr.id === id)
    if (mr && mr.portions !== desired) {
        store.adjustPortion(api, id, Math.round((desired - mr.portions) * 2) / 2)
    }
    showRecipeDetail.value = false
}

const selectedCount = computed(() => (store.menu?.menu_recipes.length || 0) + (store.menu?.custom_dishes.length || 0))

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
            <button class="btn btn-sm btn-ghost text-3xl" @click="showMenus = true; loadMenus()">
                🍽️
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
                min="1" max="8"
                class="input input-sm input-bordered w-14"
                @keyup.enter="confirmDiners"
                @blur="confirmDiners"
            />
            <div class="flex-1"></div>
            <button
                class="btn btn-sm gap-1"
                :class="selectedCount > 0 ? 'btn-accent' : 'btn-ghost'"
                @click="showSummary = !showSummary"
            >
                <span class="badge badge-sm">{{ selectedCount }}</span>道菜
            </button>
        </div>

        <!-- Category tabs -->
        <div class="flex items-center gap-1 px-3 py-2">
            <div class="flex gap-1 overflow-x-auto flex-1">
                <button
                    v-for="cat in orderedCategories"
                    :key="cat"
                    class="btn btn-sm whitespace-nowrap"
                    :class="activeCategory === cat ? 'btn-primary' : 'btn-ghost'"
                    @click="switchCategory(cat)"
                >
                    {{ cat }} <span class="badge badge-sm ml-0.5">{{ categoryCounts[cat] || 0 }}</span>
                </button>
                <button
                    class="btn btn-sm"
                    :class="activeCategory === '' ? 'btn-primary' : 'btn-ghost'"
                    @click="switchCategory('')"
                >
                    全部 <span class="badge badge-sm ml-0.5">{{ store.recipes.length }}</span>
                </button>
            </div>
            <!-- View toggle -->
            <button
                class="btn btn-sm btn-ghost flex-shrink-0"
                :title="viewMode === 'grid' ? '切换到列表视图' : '切换到网格视图'"
                @click="viewMode = viewMode === 'grid' ? 'list' : 'grid'"
            >
                {{ viewMode === 'grid' ? '☰' : '▦' }}
            </button>
        </div>

        <!-- Recipe grid -->
        <Transition :name="'slide-' + slideDirection" mode="out-in">
            <div :key="activeCategory" class="px-2 pb-24">
            <!-- Grid view -->
            <div v-if="viewMode === 'grid'" class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2">
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
            <!-- List view -->
            <div v-else class="flex flex-col gap-1">
                <div
                    v-for="r in filteredRecipes"
                    :key="r.id"
                    class="flex items-center gap-2 p-2 rounded bg-base-200 hover:bg-base-300 transition-colors cursor-pointer"
                    :class="{ 'ring-2 ring-primary': isSelected(r.id) }"
                    @click="openRecipeDetail(r.id)"
                >
                    <!-- Cover thumbnail -->
                    <img
                        v-if="r.cover_image"
                        :src="`api/image/${r.id}`"
                        :alt="r.name"
                        class="w-12 h-12 rounded object-cover flex-shrink-0"
                        loading="lazy"
                    />
                    <div v-else class="w-12 h-12 rounded bg-base-300 flex items-center justify-center text-2xl flex-shrink-0">
                        🍽️
                    </div>
                    <!-- Info -->
                    <div class="flex-1 min-w-0">
                        <div class="text-sm font-semibold truncate">{{ r.name }}</div>
                    </div>
                    <!-- Toggle button -->
                    <button
                        class="btn btn-sm flex-shrink-0"
                        :class="isSelected(r.id) ? 'btn-error btn-outline' : 'btn-primary'"
                        @click.stop="onToggle(r.id)"
                    >
                        {{ isSelected(r.id) ? '移除' : '加入' }}
                    </button>
                </div>
                <div v-if="filteredRecipes.length === 0" class="text-center text-base-content/40 py-8">
                    暂无食谱
                </div>
            </div>
        </div>
        </Transition>

        <!-- Saved menus drawer -->
        <div v-if="showMenus" class="fixed inset-0 z-50 flex" @click.self="showMenus = false">
            <div class="w-72 max-w-[80vw] bg-base-200 h-full overflow-y-auto shadow-xl p-4">
                <div class="flex items-center justify-between mb-3">
                    <h2 class="text-lg font-bold">已保存菜单</h2>
                    <button class="btn btn-sm btn-ghost" @click="showMenus = false">✕</button>
                </div>
                <div class="flex items-center gap-2 mb-3">
                    <button class="btn btn-sm btn-ghost flex-shrink-0" :disabled="syncing" @click="syncRecipes">
                        <span v-if="syncing" class="loading loading-spinner loading-xs"></span>
                        <span v-else>🔄 同步</span>
                    </button>
                    <button class="btn btn-sm btn-primary flex-1" @click="openNewMenuDialog">
                        + 新建菜单
                    </button>
                </div>
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
            <div class="flex-1 bg-black/50" @click="showMenus = false"></div>
        </div>

        <!-- Summary overlay: selected dishes → ingredients → cooking -->
        <div v-if="showSummary" class="fixed inset-0 z-50 bg-base-100 overflow-y-auto">
            <div class="max-w-lg mx-auto p-4 pb-24">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-lg font-bold">{{ selectedCount }}道菜</h2>
                    <button class="btn btn-sm btn-ghost" @click="showSummary = false">✕</button>
                </div>

                <!-- Selected dishes -->
                <div v-if="selectedDishes.length === 0 && (store.menu?.custom_dishes.length || 0) === 0" class="text-center text-base-content/40 py-8">
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

                <!-- Custom (freeform) dishes -->
                <div class="mb-4">
                    <h3 class="text-sm font-semibold text-base-content/50 mb-2">自由添加</h3>
                    <div class="space-y-2">
                        <div v-for="(cd, i) in store.menu?.custom_dishes || []" :key="'c'+i"
                            class="flex items-center gap-2 p-2 rounded bg-base-200 border border-dashed border-base-300"
                        >
                            <div class="w-7 h-7 rounded bg-accent/20 flex items-center justify-center text-xs font-bold text-accent">
                                +{{ i+1 }}
                            </div>
                            <div class="flex-1 min-w-0">
                                <div class="text-sm font-semibold">{{ cd.name }}</div>
                            </div>
                            <div class="flex items-center gap-0.5">
                                <button class="btn btn-xs btn-ghost w-5 h-5" @click.stop="adjustCustom(i, -0.5)">−</button>
                                <span class="text-xs w-6 text-center">{{ cd.portions }}</span>
                                <button class="btn btn-xs btn-ghost w-5 h-5" @click.stop="adjustCustom(i, 0.5)">+</button>
                            </div>
                            <button class="btn btn-xs btn-ghost btn-error" @click.stop="removeCustom(i)">✕</button>
                        </div>
                    </div>
                    <form class="flex gap-1 mt-2" @submit.prevent="addCustom">
                        <input
                            v-model="newCustomName"
                            type="text"
                            placeholder="输入菜名，如：随便炒个菜"
                            class="input input-sm input-bordered flex-1"
                            :disabled="addingCustom"
                        />
                        <button
                            class="btn btn-sm btn-outline btn-accent"
                            :disabled="!newCustomName.trim() || addingCustom"
                            @click="addCustom"
                        >
                            {{ addingCustom ? '...' : '添加' }}
                        </button>
                    </form>
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
                        <div class="flex items-center gap-1">
                            <button class="btn btn-sm btn-ghost w-7 h-7" @click="changeDate(-1)">◀</button>
                            <input v-model="newDate" type="date" class="input input-bordered input-sm flex-1" />
                            <button class="btn btn-sm btn-ghost w-7 h-7" @click="changeDate(1)">▶</button>
                            <span class="text-sm text-base-content/60 whitespace-nowrap">{{ dayOfWeek(newDate) }}</span>
                        </div>
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
                            <button v-for="n in 8" :key="n" class="btn btn-sm w-8 h-8"
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
                <!-- Cover image -->
                <div v-if="detailRecipe.cover_image" class="relative">
                    <img :src="`api/image/${detailRecipe.id}`" :alt="detailRecipe.name"
                        class="w-full aspect-[5/3] object-cover cursor-pointer" @click="showRecipeDetail = false" />
                    <button class="absolute top-3 right-3 btn btn-sm btn-circle btn-ghost bg-base-100/70"
                        @click="showRecipeDetail = false">✕</button>
                </div>
                <div class="px-4 pb-24 pt-3">
                    <div class="flex items-center justify-between mb-2">
                        <h1 class="text-xl font-bold">{{ detailRecipe.name }}</h1>
                        <button v-if="!detailRecipe.cover_image" class="btn btn-sm btn-circle btn-ghost"
                            @click="showRecipeDetail = false">✕</button>
                    </div>
                    <div class="flex gap-3 text-base text-base-content/60 mt-1 mb-4">
                        <span>⏱ {{ detailRecipe.cook_time }}</span>
                        <span>👥 {{ detailRecipe.servings }}人份</span>
                        <span>📊 {{ detailRecipe.difficulty }}</span>
                    </div>
                    <!-- Portion selector -->
                    <div v-if="detailRecipe.adjustable" class="flex items-center gap-2 mb-3">
                        <span class="text-sm">份量</span>
                        <button class="btn btn-xs btn-ghost w-6 h-6"
                            @click="detailAdjust(-0.5)">−</button>
                        <span class="badge badge-sm">{{ detailPortions }}份</span>
                        <button class="btn btn-xs btn-ghost w-6 h-6"
                            @click="detailAdjust(0.5)">+</button>
                        <span class="text-xs text-base-content/50">(原{{ detailRecipe.servings }}人份)</span>
                    </div>
                    <button class="btn btn-sm w-full mb-4"
                        :class="isSelected(detailRecipe.id) ? 'btn-error btn-outline' : 'btn-primary'"
                        @click="addWithPortions()">
                        {{ isSelected(detailRecipe.id) ? '从菜单中移除' : `➕ 加入菜单 (${detailPortions}份)` }}
                    </button>
                    <h2 class="text-base font-semibold mb-2">食材</h2>
                    <div class="text-base space-y-1 mb-4">
                        <template v-for="ing in detailScaledIngredients" :key="ing.name">
                            <!-- Compound ingredient group -->
                            <div v-if="ing.sub_ingredients && ing.sub_ingredients.length > 0"
                                class="rounded bg-base-200 overflow-hidden"
                            >
                                <div class="flex items-center gap-2 px-3 py-1.5 bg-base-300/50">
                                    <span class="text-sm font-bold">📦 {{ ing.name }}</span>
                                    <span v-if="ing.hint" class="text-xs text-base-content/50">{{ ing.hint }}</span>
                                </div>
                                <div class="px-2 py-1 space-y-0.5">
                                    <div v-for="(sub, j) in ing.sub_ingredients" :key="j"
                                        class="flex justify-between py-0.5 pl-4 border-b border-base-200/50"
                                    >
                                        <span class="text-sm">{{ sub.name }}</span>
                                        <span class="text-sm text-base-content/60">
                                            {{ sub.amount ? `${sub.amount}${sub.unit || ''}` : sub.hint || '' }}
                                        </span>
                                    </div>
                                </div>
                            </div>
                            <!-- Flat ingredient -->
                            <div v-else
                                class="flex justify-between py-1 border-b border-base-200">
                                <span>{{ ing.name }}</span>
                                <span class="text-base-content/60">{{ ing.amount ? `${ing.amount}${ing.unit || ''}` : ing.hint || '' }}</span>
                            </div>
                        </template>
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

.slide-left-enter-active,
.slide-left-leave-active,
.slide-right-enter-active,
.slide-right-leave-active {
    transition: transform 0.15s ease, opacity 0.15s ease;
}

.slide-left-enter-from {
    transform: translateX(60px);
    opacity: 0;
}
.slide-left-leave-to {
    transform: translateX(-60px);
    opacity: 0;
}

.slide-right-enter-from {
    transform: translateX(-60px);
    opacity: 0;
}
.slide-right-leave-to {
    transform: translateX(60px);
    opacity: 0;
}
</style>
