import { ArrowClockwise, Info } from '@phosphor-icons/react';
import { useEffect, useMemo } from 'react';
import { stringify } from 'uuid';
import {
	arraysEqual,
	ExplorerSettings,
	FilePathOrder,
	Location,
	useCache,
	useLibraryMutation,
	useLibraryQuery,
	useLibrarySubscription,
	useNodes,
	useOnlineLocations,
	useRspcLibraryContext
} from '@sd/client';
import { Loader, Tooltip } from '@sd/ui';
import { LocationIdParamsSchema } from '~/app/route-schemas';
import { Folder, Icon } from '~/components';
import {
	useIsLocationIndexing,
	useKeyDeleteFile,
	useLocale,
	useRouteTitle,
	useShortcut,
	useZodRouteParams
} from '~/hooks';
import { useQuickRescan } from '~/hooks/useQuickRescan';

import Explorer from '../Explorer';
import { ExplorerContextProvider } from '../Explorer/Context';
import {
	createDefaultExplorerSettings,
	explorerStore,
	filePathOrderingKeysSchema
} from '../Explorer/store';
import { DefaultTopBarOptions } from '../Explorer/TopBarOptions';
import { useExplorer, useExplorerSettings } from '../Explorer/useExplorer';
import { useExplorerSearchParams } from '../Explorer/util';
import { EmptyNotice } from '../Explorer/View/EmptyNotice';
import { SearchContextProvider, SearchOptions, useSearchFromSearchParams } from '../search';
import SearchBar from '../search/SearchBar';
import { useSearchExplorerQuery } from '../search/useSearchExplorerQuery';
import { TopBarPortal } from '../TopBar/Portal';
import { TOP_BAR_ICON_STYLE } from '../TopBar/TopBarOptions';
import LocationOptions from './LocationOptions';

export const Component = () => {
	const { id: locationId } = useZodRouteParams(LocationIdParamsSchema);
	const [{ path }] = useExplorerSearchParams();
	const result = useLibraryQuery(['locations.get', locationId], {
		keepPreviousData: true,
		suspense: true
	});
	useNodes(result.data?.nodes);
	const location = useCache(result.data?.item);

	// 'key' allows search state to be thrown out when entering a folder
	return <LocationExplorer key={path} location={location!} />;
};

const LocationExplorer = ({ location }: { location: Location; path?: string }) => {
	const [{ path, take }] = useExplorerSearchParams();

	const rescan = useQuickRescan();

	const { explorerSettings, preferences } = useLocationExplorerSettings(location);

	const { layoutMode, mediaViewWithDescendants, showHiddenFiles } =
		explorerSettings.useSettingsSnapshot();

	const defaultFilters = useMemo(
		() => [{ filePath: { locations: { in: [location.id] } } }],
		[location.id]
	);

	const search = useSearchFromSearchParams();

	const searchFiltersAreDefault = useMemo(
		() => JSON.stringify(defaultFilters) !== JSON.stringify(search.filters),
		[defaultFilters, search.filters]
	);

	const items = useSearchExplorerQuery({
		search,
		explorerSettings,
		filters: [
			...(search.allFilters.length > 0 ? search.allFilters : defaultFilters),
			{
				filePath: {
					path: {
						location_id: location.id,
						path: path ?? '',
						include_descendants:
							search.search !== '' ||
							(search.filters &&
								search.filters.length > 0 &&
								searchFiltersAreDefault) ||
							(layoutMode === 'media' && mediaViewWithDescendants)
					}
				}
			},
			...(!showHiddenFiles ? [{ filePath: { hidden: false } }] : [])
		],
		take,
		paths: { order: explorerSettings.useSettingsSnapshot().order },
		onSuccess: () => explorerStore.resetNewThumbnails()
	});

	const explorer = useExplorer({
		...items,
		isFetchingNextPage: items.query.isFetchingNextPage,
		isLoadingPreferences: preferences.isLoading,
		settings: explorerSettings,
		parent: { type: 'Location', location }
	});

	useLibrarySubscription(
		['locations.quickRescan', { sub_path: path ?? '', location_id: location.id }],
		{ onData() {} }
	);

	useEffect(() => {
		// Using .call to silence eslint exhaustive deps warning.
		// If clearSelectedItems referenced 'this' then this wouldn't work
		explorer.resetSelectedItems.call(undefined);
	}, [explorer.resetSelectedItems, path]);

	useEffect(() => explorer.scrollRef.current?.scrollTo({ top: 0 }), [explorer.scrollRef, path]);

	useKeyDeleteFile(explorer.selectedItems, location.id);

	useShortcut('rescan', () => rescan(location.id));

	const title = useRouteTitle(
		(path && path?.length > 1 ? getLastSectionOfPath(path) : location.name) ?? ''
	);

	const isLocationIndexing = useIsLocationIndexing(location.id);

	const { t } = useLocale();

	return (
		<ExplorerContextProvider explorer={explorer}>
			<SearchContextProvider search={search}>
				<TopBarPortal
					center={<SearchBar defaultFilters={defaultFilters} />}
					left={
						<div className="flex items-center gap-2">
							<Folder size={22} className="-mt-px" />
							<span className="truncate text-sm font-medium">{title}</span>
							<LocationOfflineInfo location={location} />
							<LocationOptions location={location} path={path || ''} />
						</div>
					}
					right={
						<DefaultTopBarOptions
							options={[
								{
									toolTipLabel: t('reload'),
									onClick: () => rescan(location.id),
									icon: <ArrowClockwise className={TOP_BAR_ICON_STYLE} />,
									individual: true,
									showAtResolution: 'xl:flex'
								}
							]}
						/>
					}
				>
					{search.open && (
						<>
							<hr className="w-full border-t border-sidebar-divider bg-sidebar-divider" />
							<SearchOptions />
						</>
					)}
				</TopBarPortal>
			</SearchContextProvider>
			{isLocationIndexing ? (
				<div className="flex size-full items-center justify-center">
					<Loader />
				</div>
			) : !preferences.isLoading ? (
				<Explorer
					emptyNotice={
						<EmptyNotice
							icon={<Icon name="FolderNoSpace" size={128} />}
							message={t('location_empty_notice_message')}
						/>
					}
				/>
			) : null}
		</ExplorerContextProvider>
	);
};

function LocationOfflineInfo({ location }: { location: Location }) {
	const onlineLocations = useOnlineLocations();

	const locationOnline = useMemo(
		() => onlineLocations.some((l) => arraysEqual(location.pub_id, l)),
		[location.pub_id, onlineLocations]
	);

	const { t } = useLocale();

	return (
		<>
			{!locationOnline && (
				<Tooltip label={t('location_disconnected_tooltip')}>
					<Info className="text-ink-faint" />
				</Tooltip>
			)}
		</>
	);
}

function getLastSectionOfPath(path: string): string | undefined {
	if (path.endsWith('/')) {
		path = path.slice(0, -1);
	}
	const sections = path.split('/');
	const lastSection = sections[sections.length - 1];
	return lastSection;
}

function useLocationExplorerSettings(location: Location) {
	const rspc = useRspcLibraryContext();

	const preferences = useLibraryQuery(['preferences.get']);
	const updatePreferences = useLibraryMutation('preferences.update');

	const settings = useMemo(() => {
		const defaults = createDefaultExplorerSettings<FilePathOrder>({
			order: { field: 'name', value: 'Asc' }
		});

		if (!location) return defaults;

		const pubId = stringify(location.pub_id);

		const settings = preferences.data?.location?.[pubId]?.explorer;

		if (!settings) return defaults;

		for (const [key, value] of Object.entries(settings)) {
			if (value !== null) Object.assign(defaults, { [key]: value });
		}

		return defaults;
	}, [location, preferences.data?.location]);

	const onSettingsChanged = async (
		settings: ExplorerSettings<FilePathOrder>,
		changedLocation: Location
	) => {
		if (changedLocation.id === location.id && preferences.isLoading) return;

		const pubId = stringify(changedLocation.pub_id);

		try {
			await updatePreferences.mutateAsync({
				location: { [pubId]: { explorer: settings } }
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
			orderingKeys: filePathOrderingKeysSchema,
			data: location
		}),
		preferences
	};
}
