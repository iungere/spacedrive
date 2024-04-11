import { useMemo } from 'react';
import {
	ExplorerSettings,
	ObjectOrder,
	Tag,
	useCache,
	useLibraryMutation,
	useLibraryQuery,
	useNodes,
	useRspcLibraryContext
} from '@sd/client';
import { LocationIdParamsSchema } from '~/app/route-schemas';
import { Icon } from '~/components';
import { useLocale, useRouteTitle, useZodRouteParams } from '~/hooks';
import { stringify } from '~/util/uuid';

import Explorer from '../Explorer';
import { ExplorerContextProvider } from '../Explorer/Context';
import { createDefaultExplorerSettings, objectOrderingKeysSchema } from '../Explorer/store';
import { DefaultTopBarOptions } from '../Explorer/TopBarOptions';
import { useExplorer, useExplorerSettings } from '../Explorer/useExplorer';
import { EmptyNotice } from '../Explorer/View/EmptyNotice';
import { SearchContextProvider, SearchOptions, useSearchFromSearchParams } from '../search';
import SearchBar from '../search/SearchBar';
import { useSearchExplorerQuery } from '../search/useSearchExplorerQuery';
import { TopBarPortal } from '../TopBar/Portal';

export function Component() {
	const { id: tagId } = useZodRouteParams(LocationIdParamsSchema);
	const result = useLibraryQuery(['tags.get', tagId], { suspense: true });
	useNodes(result.data?.nodes);
	const tag = useCache(result.data?.item);

	const { t } = useLocale();

	useRouteTitle(tag!.name ?? 'Tag');

	const { explorerSettings, preferences } = useTagExplorerSettings(tag);

	const search = useSearchFromSearchParams();

	const defaultFilters = useMemo(() => [{ object: { tags: { in: [tag.id] } } }], [tag.id]);

	const items = useSearchExplorerQuery({
		search,
		explorerSettings,
		filters: search.allFilters.length > 0 ? search.allFilters : defaultFilters,
		take: 100,
		objects: { order: explorerSettings.useSettingsSnapshot().order }
	});

	const explorer = useExplorer({
		...items,
		isFetchingNextPage: items.query.isFetchingNextPage,
		isLoadingPreferences: preferences.isLoading,
		settings: explorerSettings,
		parent: { type: 'Tag', tag: tag! }
	});

	return (
		<ExplorerContextProvider explorer={explorer}>
			<SearchContextProvider search={search}>
				<TopBarPortal
					center={<SearchBar defaultFilters={defaultFilters} defaultTarget="objects" />}
					left={
						<div className="flex flex-row items-center gap-2">
							<div
								className="size-[14px] shrink-0 rounded-full"
								style={{ backgroundColor: tag!.color || '#efefef' }}
							/>
							<span className="truncate text-sm font-medium">{tag?.name}</span>
						</div>
					}
					right={<DefaultTopBarOptions />}
				>
					{search.open && (
						<>
							<hr className="w-full border-t border-sidebar-divider bg-sidebar-divider" />
							<SearchOptions />
						</>
					)}
				</TopBarPortal>
			</SearchContextProvider>

			{!preferences.isLoading && (
				<Explorer
					emptyNotice={
						<EmptyNotice
							icon={<Icon name="Tags" size={128} />}
							message={t('tags_notice_message')}
						/>
					}
				/>
			)}
		</ExplorerContextProvider>
	);
}

function useTagExplorerSettings(tag: Tag) {
	const rspc = useRspcLibraryContext();

	const preferences = useLibraryQuery(['preferences.get']);
	const updatePreferences = useLibraryMutation('preferences.update');

	const settings = useMemo(() => {
		const defaults = createDefaultExplorerSettings<ObjectOrder>({ order: null });

		if (!location) return defaults;

		const pubId = stringify(tag.pub_id);

		const settings = preferences.data?.location?.[pubId]?.explorer;

		if (!settings) return defaults;

		for (const [key, value] of Object.entries(settings)) {
			if (value !== null) Object.assign(defaults, { [key]: value });
		}

		return defaults;
	}, [tag, preferences.data?.location]);

	const onSettingsChanged = async (settings: ExplorerSettings<ObjectOrder>, changedTag: Tag) => {
		if (changedTag.id === tag.id && preferences.isLoading) return;

		const pubId = stringify(changedTag.pub_id);

		try {
			await updatePreferences.mutateAsync({
				tag: { [pubId]: { explorer: settings } }
			});
			rspc.queryClient.invalidateQueries(['preferences.get']);
		} catch (e) {
			alert('An error has occurred while updating your preferences.');
		}
	};

	return {
		explorerSettings: useExplorerSettings({
			settings,
			onSettingsChanged,
			orderingKeys: objectOrderingKeysSchema,
			data: tag
		}),
		preferences
	};
}
