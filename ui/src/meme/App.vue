<script setup lang="ts">
import { getCurrentInstance, onMounted, Ref, ref, watch } from 'vue';
import Waterfall from './pages/Waterfall.vue';
import Gallery from './pages/Gallery.vue';
import { raven } from './raven';
import { MemeList, Meta, PageType as PageType, SortKey } from './lib/struct';
import { debounce } from 'lodash';
import { useConfigStore } from './lib/configStore';
import { storeToRefs } from 'pinia';
import Upload from './pages/Upload.vue';
import FullScreenPreview from './pages/FullScreenPreview.vue';

const config = useConfigStore();
const { page: page, waterfall_pagnition } = storeToRefs(config);
const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
let token: Ref<string | null> = ref(null);
let name: Ref<string | null> = ref(null);
let data: Ref<MemeList | null> = ref(null);
let filter: Ref<string> = ref('');
let is_asc: Ref<boolean> = ref(false);
let sort = ref(SortKey.timestamp);
let single_preview: Ref<Meta | null> = ref(null);

const update = debounce(async function update() {
    if (!token.value) {
        data.value = null;
        return;
    }
    try {
        let res = await api?.get('list', {
            headers: { token: token.value },
            params: { filter: filter.value, asc: is_asc.value, sort: `${sort.value}` },
        });
        data.value = res?.data;
    } catch {
        data.value = null;
    }
}, 500);

async function logout() {
    filter.value = '';
    token.value = null;
    data.value = null;
}

function show(meta: Meta) {
    single_preview.value = meta;
}

function uploaded(meta: Meta) {
    update();
    console.log(`uploaded ${meta.uuid} with tags: ${meta.tags}`);
    show(meta);
}

onMounted(() => {
    if (localStorage.getItem('token')?.length ?? 0 > 0) {
        let temp = localStorage.getItem('token');
        if (temp) {
            const payload = JSON.parse(atob(temp.split('.')[1]));
            if (payload.exp > Date.now() / 1000) {
                token.value = temp;
            }
        }
    }
});

watch(token, (newVal) => {
    if (newVal) {
        const payload = JSON.parse(atob(newVal.split('.')[1]));
        name.value = payload.n;
        console.log('new name: ', name.value);
        document.cookie = `token=${newVal}`;
        update();
    } else {
        document.cookie = `token=`;
        name.value = null;
    }
});

watch([filter, is_asc, sort], update);
</script>

<template>
    <div class="fixed navbar bg-base-100 z-50 opacity-0 hover:opacity-90 rounded-3xl">
        <div class="flex-1 min-w-0">
            <input type="text" placeholder="" class="input input-ghost w-full" v-model="filter" />
        </div>
        <div v-if="token" class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-ghost">
                <div class="w-2">↓</div>
            </div>
            <ul tabindex="0" class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-80 p-2 shadow">
                <li>
                    <span>user</span>
                    <ul>
                        <li>
                            <a>{{ name }}</a>
                        </li>
                        <li><a @click="logout">logout</a></li>
                        <li><a @click="update">update</a></li>
                    </ul>
                </li>
                <li>
                    <span>config</span>
                    <ul>
                        <li>
                            <span>general</span>
                            <ul>
                                <li>
                                    <div class="flex" @click="() => (is_asc = !is_asc)">
                                        <span>order</span>
                                        <span>{{ is_asc ? '↑' : '↓' }}</span>
                                    </div>
                                </li>
                                <li>
                                    <details>
                                        <summary>sort by {{ sort }}</summary>
                                        <ul>
                                            <li>
                                                <span @click="() => (sort = SortKey.timestamp)">
                                                    {{ SortKey.timestamp }}
                                                </span>
                                            </li>
                                            <li>
                                                <span @click="() => (sort = SortKey.uuid)">
                                                    {{ SortKey.uuid }}
                                                </span>
                                            </li>
                                        </ul>
                                    </details>
                                </li>
                            </ul>
                        </li>
                        <li>
                            <span>page</span>
                            <ul>
                                <li>
                                    <div @click="() => (page = PageType.Waterfall)">
                                        <input
                                            type="radio"
                                            class="radio"
                                            :checked="page == PageType.Waterfall"
                                            @click="() => (is_asc = !is_asc)"
                                        />
                                        <span>waterfall</span>
                                    </div>
                                </li>
                                <li>
                                    <div class="flex" @click="() => (page = PageType.Gallery)">
                                        <input type="radio" class="radio" :checked="page == PageType.Gallery" />
                                        <span>gallery</span>
                                    </div>
                                </li>
                            </ul>
                        </li>
                    </ul>
                </li>
                <li v-if="page == PageType.Waterfall">
                    <span>range</span>
                    <ul>
                        <li>
                            <div>
                                <input
                                    type="range"
                                    class="range range-xs"
                                    min="10"
                                    max="50"
                                    step="5"
                                    v-model.number="waterfall_pagnition"
                                />
                                <div>{{ waterfall_pagnition }}</div>
                            </div>
                        </li>
                    </ul>
                </li>
            </ul>
        </div>
        <div
            v-else
            class="btn btn-ghost"
            @click="
                () => {
                    raven('https://hodor.32323235.xyz/').then((res) => (token = res));
                }
            "
        >
            ⊙
        </div>
    </div>
    <Waterfall v-if="token && page == PageType.Waterfall" class="main-entry-container" :data="data" @show="show"> </Waterfall>
    <Gallery v-else-if="token && page == PageType.Gallery" class="main-entry-container" :data="data"></Gallery>
    <Upload v-if="token && !single_preview" @done="uploaded"></Upload>
    <FullScreenPreview v-if="token && single_preview" :meta="single_preview" @end="() => (single_preview = null)"></FullScreenPreview>
</template>

<style scoped lang="postcss"></style>
