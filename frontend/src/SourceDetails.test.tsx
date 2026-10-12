import {render,screen} from '@testing-library/react';
import {expect,test} from 'vitest';
import SourceDetails from './SourceDetails';
test('safe readable fields retain historical UTC dates and unspecified season',()=>{
 render(<SourceDetails sourceKey="Leaf_Collection_Sections/0" details={{DESCRIPT:'October 5–9',LASTUPDATE:1601510400000,NOTES:'private'}}/>);
 expect(screen.getByText('Published collection windows')).toBeVisible();
 expect(screen.getByText(/year is unspecified/)).toBeVisible();
 expect(screen.getByText(/2020-10-01/)).toBeVisible();
 expect(screen.queryByText('private')).not.toBeInTheDocument();
});
